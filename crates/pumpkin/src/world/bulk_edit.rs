//! Bulk block edits requested by plugins (`get_block_state_ids` / `set_block_state_ids`).
//!
//! The heavy work runs on a dedicated, named rayon pool (`ArkEdit-N`, size from
//! `plugins.world_edit_threads`, default `max(1, cores - 2)`, low priority). It never uses the
//! tick thread, the async runtime workers or the global rayon pool.
//!
//! A bulk write is done in four phases:
//! 1. the raw block writes, grouped per chunk and done in parallel (`set_blocks_batch`);
//! 2. the per-block side effects (block entities, `placed` callbacks, neighbour/shape updates,
//!    points of interest), in parallel per chunk, exactly the code of [`World::set_block_state`]
//!    that follows the raw write. They see the final state of the whole batch, like `/fill`;
//! 3. the light update, in ONE serial pass at the end (the light engine queues are shared by the
//!    whole level and are not thread safe). Light checks of nested updates are collected too;
//! 4. the client updates, one `CMultiBlockUpdate` per chunk section, sent from the pool.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use pumpkin_data::{Block, BlockStateId};
use pumpkin_util::math::{
    position::{BlockPos, chunk_section_from_pos},
    vector2::Vector2,
    vector3::Vector3,
};
use pumpkin_world::chunk::io::Dirtiable;
use pumpkin_world::world::BlockFlags;
use rayon::prelude::*;

use super::{World, villager_poi};

/// Wraps every task that runs on the pool (used to propagate the plugin reentry context, so
/// that a plugin event fired from a pool thread behaves like one fired from the calling thread).
pub type TaskScope = dyn Fn(&mut dyn FnMut()) + Send + Sync;

static EDIT_POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();

/// The dedicated pool for bulk edits. `configured == 0` means `max(1, cores - 2)`.
pub fn edit_pool(configured: usize) -> &'static rayon::ThreadPool {
    EDIT_POOL.get_or_init(|| {
        let threads = if configured == 0 {
            std::thread::available_parallelism()
                .map_or(2, std::num::NonZero::get)
                .saturating_sub(2)
                .max(1)
        } else {
            configured
        };
        tracing::info!("Bulk world edit pool: {threads} thread(s) (ArkEdit-N)");
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|index| format!("ArkEdit-{index}"))
            .start_handler(|_| lower_thread_priority())
            .build()
            .expect("the bulk edit thread pool could not be created")
    })
}

/// Nice +5 for the calling thread so that the tick thread and the network keep the cores.
fn lower_thread_priority() {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: `gettid` and `setpriority` only read their integer arguments; a failure is
        // reported through the return value, which is deliberately ignored.
        unsafe {
            let tid = libc::syscall(libc::SYS_gettid) as libc::id_t;
            libc::setpriority(libc::PRIO_PROCESS, tid, 5);
        }
    }
}

fn scoped<T>(scope: &TaskScope, work: impl FnOnce() -> T) -> T {
    let mut work = Some(work);
    let mut out = None;
    scope(&mut || {
        if let Some(work) = work.take() {
            out = Some(work());
        }
    });
    out.expect("the task scope did not run the task")
}

// Light checks of the block changes made by the current pool task (see `defer_light`).
thread_local! {
    static DEFERRED_LIGHT: RefCell<Option<(usize, Vec<BlockPos>)>> = const { RefCell::new(None) };
}

/// Called by [`World::set_block_state`] where it would update the light: when the current thread
/// is inside a bulk edit of that world, the position is collected for the final light pass
/// (and `true` is returned) instead of running a light update now.
pub fn defer_light(world: &World, pos: BlockPos) -> bool {
    DEFERRED_LIGHT.with(|cell| {
        if let Some((owner, list)) = cell.borrow_mut().as_mut()
            && *owner == std::ptr::from_ref(world) as usize
        {
            list.push(pos);
            return true;
        }
        false
    })
}

struct DeferGuard;

impl DeferGuard {
    fn begin(world: &World) -> Self {
        DEFERRED_LIGHT.with(|cell| {
            *cell.borrow_mut() = Some((std::ptr::from_ref(world) as usize, Vec::new()));
        });
        Self
    }

    fn take(self) -> Vec<BlockPos> {
        DEFERRED_LIGHT
            .with(|cell| cell.borrow_mut().take())
            .map(|(_, list)| list)
            .unwrap_or_default()
    }
}

impl Drop for DeferGuard {
    fn drop(&mut self) {
        DEFERRED_LIGHT.with(|cell| *cell.borrow_mut() = None);
    }
}

struct Applied {
    pos: BlockPos,
    old: BlockStateId,
    new: BlockStateId,
}

impl World {
    /// Reads many block states, in parallel per chunk. `None` if a chunk is not in memory
    /// (or a position is outside the build limit): the caller loads the chunks and retries.
    pub fn get_block_state_ids_bulk(
        &self,
        pool: &rayon::ThreadPool,
        scope: &TaskScope,
        positions: &[(i32, i32, i32)],
    ) -> Option<Vec<u16>> {
        let mut groups: HashMap<Vector2<i32>, Vec<usize>> = HashMap::new();
        for (index, (x, y, z)) in positions.iter().enumerate() {
            if !self.is_in_build_limit(BlockPos::new(*x, *y, *z)) {
                return None;
            }
            groups
                .entry(Vector2::new(x.div_euclid(16), z.div_euclid(16)))
                .or_default()
                .push(index);
        }
        let groups: Vec<(Vector2<i32>, Vec<usize>)> = groups.into_iter().collect();
        let parts: Vec<Option<Vec<(usize, u16)>>> = pool.install(|| {
            groups
                .par_iter()
                .map(|(chunk_pos, indices)| {
                    scoped(scope, || {
                        let chunk = self
                            .level
                            .loaded_chunks
                            .get(chunk_pos)
                            .map(|entry| entry.value().clone())?;
                        let mut out = Vec::with_capacity(indices.len());
                        for &index in indices {
                            let (x, y, z) = positions[index];
                            let state = chunk.section.get_block_absolute_y(
                                x.rem_euclid(16) as usize,
                                y,
                                z.rem_euclid(16) as usize,
                            )?;
                            out.push((index, state.as_u16()));
                        }
                        Some(out)
                    })
                })
                .collect()
        });
        let mut result = vec![0u16; positions.len()];
        for part in parts {
            for (index, state) in part? {
                result[index] = state;
            }
        }
        Some(result)
    }

    /// Writes many block states with the same observable result as calling
    /// [`World::set_block_state`] for each change (same final states, same callbacks, same
    /// packets), but in parallel and with one light pass. Returns `false`, having changed
    /// nothing, if a chunk is not in memory.
    pub fn set_block_states_bulk(
        self: &Arc<Self>,
        pool: &rayon::ThreadPool,
        scope: &TaskScope,
        changes: Vec<(BlockPos, BlockStateId)>,
        flags: BlockFlags,
    ) -> bool {
        let profile = std::env::var_os("ARKEDIT_PROFILE").is_some();
        let t0 = std::time::Instant::now();
        let total = changes.len();
        // Positions outside the build limit are ignored, like `set_block_state` does.
        type Update = (usize, i32, usize, BlockStateId);
        let mut groups: HashMap<Vector2<i32>, Vec<Update>> = HashMap::new();
        for (pos, state) in changes {
            if !self.is_in_build_limit(pos) {
                continue;
            }
            let (chunk, relative) = pos.chunk_and_chunk_relative_position();
            groups.entry(chunk).or_default().push((
                relative.x as usize,
                pos.0.y,
                relative.z as usize,
                state,
            ));
        }
        let mut chunks = Vec::with_capacity(groups.len());
        for (chunk_pos, updates) in groups {
            let Some(chunk) = self
                .level
                .loaded_chunks
                .get(&chunk_pos)
                .map(|entry| entry.value().clone())
            else {
                return false;
            };
            chunks.push((chunk_pos, chunk, updates));
        }

        let t_group = t0.elapsed();
        // Phase 1: raw writes, one task per chunk.
        let written: Vec<Vec<Applied>> = pool.install(|| {
            chunks
                .into_par_iter()
                .map(|(chunk_pos, chunk, updates)| {
                    scoped(scope, || {
                        let new_of: Vec<BlockStateId> =
                            updates.iter().map(|update| update.3).collect();
                        let replaced = chunk.set_blocks_batch(updates);
                        // One result per update, in order, unless an update was out of the
                        // chunk's sections: then the new state is read back.
                        let aligned = replaced.len() == new_of.len();
                        let mut out = Vec::with_capacity(replaced.len());
                        let mut changed = false;
                        for (index, (rx, y, rz, old)) in replaced.into_iter().enumerate() {
                            let new = if aligned {
                                new_of[index]
                            } else {
                                chunk
                                    .section
                                    .get_block_absolute_y(rx, y, rz)
                                    .unwrap_or(old)
                            };
                            changed |= old != new;
                            out.push(Applied {
                                pos: BlockPos::new(
                                    chunk_pos.x * 16 + rx as i32,
                                    y,
                                    chunk_pos.y * 16 + rz as i32,
                                ),
                                old,
                                new,
                            });
                        }
                        if changed && !chunk.is_dirty() {
                            chunk.mark_dirty(true);
                        }
                        out
                    })
                })
                .collect()
        });

        let t_write = t0.elapsed();
        // Phase 2: side effects, one task per chunk, light checks collected.
        struct Outcome {
            to_send: Vec<(BlockPos, BlockStateId)>,
            light: Vec<BlockPos>,
        }
        let outcomes: Vec<Outcome> = pool.install(|| {
            written
                .into_par_iter()
                .map(|applied| {
                    scoped(scope, || {
                        let guard = DeferGuard::begin(self);
                        let mut to_send = Vec::new();
                        let mut light = Vec::new();
                        for change in applied {
                            if !flags.contains(BlockFlags::FORCE_STATE) && change.old == change.new
                            {
                                continue;
                            }
                            if self.apply_block_change_effects(
                                &change.pos,
                                change.old,
                                change.new,
                                flags,
                            ) {
                                to_send.push((change.pos, change.new));
                            }
                            if pumpkin_world::lighting::LightEngine::has_different_light_properties(
                                change.old.to_state(),
                                change.new.to_state(),
                            ) {
                                light.push(change.pos);
                            }
                        }
                        light.extend(guard.take());
                        Outcome { to_send, light }
                    })
                })
                .collect()
        });

        let t_effects = t0.elapsed();
        let mut light: Vec<BlockPos> = Vec::new();
        let mut by_section: HashMap<Vector3<i32>, Vec<(BlockPos, BlockStateId)>> = HashMap::new();
        for outcome in outcomes {
            light.extend(outcome.light);
            for (pos, state) in outcome.to_send {
                by_section
                    .entry(chunk_section_from_pos(&pos))
                    .or_default()
                    .push((pos, state));
            }
        }

        // Phase 3: one serial light pass (column tops are computed in parallel first).
        if !light.is_empty() {
            let columns: Vec<(i32, i32)> = {
                let mut seen: Vec<(i32, i32)> = light.iter().map(|p| (p.0.x, p.0.z)).collect();
                seen.sort_unstable();
                seen.dedup();
                seen
            };
            let tops: HashMap<(i32, i32), i32> = pool.install(|| {
                columns
                    .into_par_iter()
                    .map(|(x, z)| {
                        (
                            (x, z),
                            self.level.light_engine.sky_blocking_top(&self.level, x, z),
                        )
                    })
                    .collect()
            });
            self.level
                .light_engine
                .update_lighting_batch(&self.level, &light, &tops);
        }

        let t_light = t0.elapsed();
        // Phase 4: client updates, one multi block update per chunk section.
        let sections: Vec<(Vector3<i32>, Vec<(BlockPos, BlockStateId)>)> =
            by_section.into_iter().collect();
        pool.install(|| {
            sections.into_par_iter().for_each(|(section, updates)| {
                scoped(scope, || self.send_section_updates(section, updates));
            });
        });
        if profile {
            tracing::info!(
                "bulk edit of {total}: group {t_group:?}, write {t_write:?}, effects {t_effects:?}, light {t_light:?}, total {:?}",
                t0.elapsed()
            );
        }
        true
    }

    /// Everything [`World::set_block_state`] does after the raw write of a block, except the
    /// light update and the queuing of the client update. Returns whether the client must be
    /// told about the new state (`NOTIFY_LISTENERS` and the state is still the one that was set).
    fn apply_block_change_effects(
        self: &Arc<Self>,
        position: &BlockPos,
        replaced_block_state_id: BlockStateId,
        block_state_id: BlockStateId,
        flags: BlockFlags,
    ) -> bool {
        let old_block = Block::from_state_id(replaced_block_state_id);
        let new_block = Block::from_state_id(block_state_id);
        let is_new_block = old_block != new_block;
        let block_moved = flags.contains(BlockFlags::MOVED);

        if is_new_block
            && old_block.default_state.block_entity_type != u16::MAX
            && let Some(entity) = self.get_block_entity(position)
        {
            if !flags.contains(BlockFlags::SKIP_BLOCK_ENTITY_REPLACED_CALLBACK) {
                entity.on_block_replaced(self, position);
            }
            self.remove_block_entity(position);
        }

        if is_new_block && (flags.contains(BlockFlags::NOTIFY_NEIGHBORS) || block_moved) {
            self.block_registry.on_state_replaced(
                self,
                old_block,
                position,
                replaced_block_state_id,
                block_moved,
            );
        }

        if !flags.contains(BlockFlags::SKIP_BLOCK_ADDED_CALLBACK) && is_new_block {
            self.block_registry.on_placed(
                self,
                new_block,
                block_state_id,
                position,
                replaced_block_state_id,
                block_moved,
            );
            let new_fluid = self.get_fluid(position);
            self.block_registry.on_placed_fluid(
                self,
                new_fluid,
                block_state_id,
                position,
                replaced_block_state_id,
                block_moved,
            );
        }

        if self.get_block_state_id(position) != block_state_id {
            return false;
        }

        if flags.contains(BlockFlags::NOTIFY_NEIGHBORS) {
            self.update_neighbors_at(position, old_block, None);
            if block_state_id.has_analog_output_signal() {
                self.update_neighbour_for_output_signal(position, new_block);
            }
        }

        if !flags.contains(BlockFlags::MOVED) {
            let mut neighbour_update_flags = flags;
            neighbour_update_flags.remove(BlockFlags::NOTIFY_NEIGHBORS);
            neighbour_update_flags.remove(BlockFlags::SKIP_DROPS);
            self.block_registry.prepare(
                self,
                position,
                old_block,
                replaced_block_state_id,
                neighbour_update_flags,
            );
            self.block_registry
                .update_neighbors(self, position, neighbour_update_flags);
            self.block_registry.prepare(
                self,
                position,
                new_block,
                block_state_id,
                neighbour_update_flags,
            );
        }

        // The points of interest are only touched when a job site or a portal block is involved.
        if villager_poi::profession_for_block(new_block).is_some()
            || villager_poi::profession_for_block(old_block).is_some()
        {
            self.villager_poi
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .update_block(*position, new_block);
        }

        if is_new_block {
            let relevant = villager_poi::profession_for_block(old_block).is_some()
                || villager_poi::poi_type_for_block(new_block).is_some();
            if relevant {
                let mut poi = self
                    .portal_poi
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if villager_poi::profession_for_block(old_block).is_some() {
                    poi.remove(position);
                }
                if let Some(poi_type) = villager_poi::poi_type_for_block(new_block) {
                    poi.add_with_free_tickets(*position, poi_type, 1);
                }
            }
        }

        flags.contains(BlockFlags::NOTIFY_LISTENERS)
    }
}
