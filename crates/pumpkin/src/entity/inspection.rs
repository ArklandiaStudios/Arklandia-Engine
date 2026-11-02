//! Live player inventory views. Slots reference owner storage; there is no copy-back snapshot.
use std::{any::Any, sync::{Arc, Mutex, atomic::{AtomicU8, Ordering}}};
use pumpkin_data::{item::Item, item_stack::ItemStack, screen::WindowType, data_component_impl::{CustomNameImpl, EquipmentSlot}};
use pumpkin_inventory::{Inventory, Clearable, player::player_inventory::PlayerInventory,
    screen_handler::{InventoryPlayer, ScreenHandler, ScreenHandlerBehaviour, ScreenHandlerFactory, SharedScreenHandler}, slot::{Slot, NormalSlot, ArmorSlot}};
use pumpkin_util::text::TextComponent;
use super::player::Player;

fn owner_slot(index: usize, ender: bool) -> Option<usize> {
    if ender { return (index < 27).then_some(index); }
    match index { 0..=35 => Some(index), 40 => Some(40), 41 => Some(36), 42 => Some(37), 43 => Some(38), 44 => Some(39), _ => None }
}
fn blocked_slot_placeholder() -> ItemStack {
    let mut stack = ItemStack::new(1, &Item::BLACK_STAINED_GLASS_PANE);
    stack.set_data_component(CustomNameImpl { name: TextComponent::text("Case inutilisable") });
    stack
}
struct LiveInventory { target: Arc<Player>, ender: bool }
impl LiveInventory {
    fn storage(&self) -> Arc<dyn Inventory> {
        if self.ender { self.target.ender_chest_inventory().clone() } else { self.target.inventory.clone() }
    }
    fn writable(&self, index: usize) -> bool {
        owner_slot(index, self.ender).is_some()
            && (self.ender || index != 8 || !self.target.silent_container_opening.load(Ordering::Relaxed))
    }
    fn changed(&self, slot: usize) {
        self.target.sync_inventory_to_client();
        if !self.ender {
            let equipment = self.target.inventory.equipment_slots.get(&slot).cloned().or_else(|| {
                (slot == self.target.inventory.get_selected_slot() as usize).then_some(EquipmentSlot::MAIN_HAND)
            });
            if let Some(equipment) = equipment {
                self.target.living_entity.send_equipment_changes(&[(equipment, self.target.inventory.get_stack(slot))]);
                self.target.living_entity.send_current_equipment_attribute_modifiers();
            }
        }
    }
}
impl Clearable for LiveInventory {
    fn clear(&self) { for i in 0..self.size() { self.set_stack(i, ItemStack::EMPTY.clone()); } }
}
impl Inventory for LiveInventory {
    fn size(&self) -> usize { if self.ender {27} else {45} }
    fn is_empty(&self) -> bool { (0..self.size()).all(|i| self.get_stack(i).is_empty()) }
    fn get_stack(&self, slot: usize) -> ItemStack {
        // These four display-only stacks never enter owner storage. All write,
        // take and insert paths remain blocked by writable()/LiveSlot.
        if !self.ender && (36..=39).contains(&slot) { return blocked_slot_placeholder(); }
        owner_slot(slot,self.ender).map_or_else(||ItemStack::EMPTY.clone(), |i|self.storage().get_stack(i))
    }
    fn set_stack(&self, slot: usize, stack: ItemStack) {
        if self.writable(slot) { if let Some(i)=owner_slot(slot,self.ender) {self.storage().set_stack(i,stack);self.changed(i);} }
    }
    fn remove_stack(&self, slot: usize) -> ItemStack {
        if self.writable(slot) { if let Some(i)=owner_slot(slot,self.ender) {let s=self.storage().remove_stack(i);self.changed(i);return s;} }
        ItemStack::EMPTY.clone()
    }
    fn remove_stack_specific(&self, slot: usize, amount: u8) -> ItemStack {
        if self.writable(slot) {if let Some(i)=owner_slot(slot,self.ender) {let s=self.storage().remove_stack_specific(i,amount);self.changed(i);return s;} }
        ItemStack::EMPTY.clone()
    }
    fn as_any(&self) -> &dyn Any {self}
}
struct LiveSlot { inventory: Arc<LiveInventory>, index: usize, id: AtomicU8 }
impl Slot for LiveSlot {
    fn get_inventory(&self) -> Arc<dyn Inventory> {self.inventory.clone()}
    fn get_index(&self) -> usize {self.index}
    fn set_id(&self,id: usize) {self.id.store(id as u8,Ordering::Relaxed);}
    fn mark_dirty(&self) {}
    fn can_take_items(&self,_player:&dyn InventoryPlayer)->bool {self.inventory.writable(self.index)}
    fn can_insert(&self,stack:&ItemStack)->bool {
        if !self.inventory.writable(self.index) {return false;}
        if !self.inventory.ender && (41..=44).contains(&self.index) {
            let equipment=match self.index {41=>EquipmentSlot::FEET,42=>EquipmentSlot::LEGS,43=>EquipmentSlot::CHEST,_=>EquipmentSlot::HEAD};
            return ArmorSlot::new(self.inventory.clone(),self.index,equipment).can_insert(stack);
        }
        true
    }
    // Vanilla's merge pass checks capacity before can_insert. Zero capacity
    // also prevents a matching named pane from being merged into the display.
    fn get_max_item_count(&self)->u8 {if !self.inventory.writable(self.index) {0} else if !self.inventory.ender && (41..=44).contains(&self.index) {1} else {99}}
}
pub struct InspectionScreen {
    pub target: Arc<Player>,
    ender: bool,
    behaviour: ScreenHandlerBehaviour,
}
impl InspectionScreen {
    pub fn protected_slot(&self,slot:i32)->bool {
        !self.ender && ((36..=39).contains(&slot) || (slot==8 && self.target.silent_container_opening.load(Ordering::Relaxed)))
    }
}
impl ScreenHandler for InspectionScreen {
    fn as_any(&self)->&dyn Any {self}
    fn as_any_mut(&mut self)->&mut dyn Any {self}
    fn get_behaviour(&self)->&ScreenHandlerBehaviour {&self.behaviour}
    fn get_behaviour_mut(&mut self)->&mut ScreenHandlerBehaviour {&mut self.behaviour}
    fn can_use(&self,player:&dyn InventoryPlayer)->bool {
        let Some(viewer)=player.as_any().downcast_ref::<Player>() else {return false;};
        let world=self.target.world();
        let Some(server)=world.server.upgrade() else {return false;};
        let Some(target)=server.get_player_by_uuid(self.target.gameprofile.id) else {return false;};
        let Some(viewer)=server.get_player_by_uuid(viewer.gameprofile.id) else {return false;};
        Arc::ptr_eq(&target,&self.target) && !target.client.closed() && !target.living_entity.dead.load(Ordering::Relaxed)
            && (!target.silent_container_opening.load(Ordering::Relaxed) || viewer.has_permission(&server,"arkstaff.vanish.see"))
            && viewer.has_permission(&server, if self.ender {"ArkStaff:command.endersee"} else {"ArkStaff:command.invsee"})
    }
    fn quick_move(&mut self,player:&dyn InventoryPlayer,index:i32)->ItemStack {
        let Some(slot)=self.behaviour.slots.get(index as usize).cloned() else {return ItemStack::EMPTY.clone();};
        if !slot.can_take_items(player) {return ItemStack::EMPTY.clone();}
        let mut stack=slot.get_stack();let original=stack.clone();
        let count=self.behaviour.container_slots as i32;
        let (start,end,reverse)=if index<count {(count,self.behaviour.slots.len() as i32,true)} else {(0,count,false)};
        if stack.is_empty() || !self.insert_item(&mut stack,start,end,reverse) {return ItemStack::EMPTY.clone();}
        slot.set_stack(stack);slot.on_take_item(player,&original);original
    }
}
pub struct InspectionFactory { pub target: Arc<Player>, pub ender: bool, pub title: TextComponent }
impl ScreenHandlerFactory for InspectionFactory {
    fn get_display_name(&self)->TextComponent {self.title.clone()}
    fn create_screen_handler(&self,sync_id:u8,player_inventory:&Arc<PlayerInventory>,_player:&dyn InventoryPlayer)->Option<SharedScreenHandler> {
        let count=if self.ender {27} else {45};
        let inventory=Arc::new(LiveInventory{target:self.target.clone(),ender:self.ender});
        let mut handler=InspectionScreen{target:self.target.clone(),ender:self.ender,behaviour:ScreenHandlerBehaviour::new(sync_id,Some(if self.ender {WindowType::Generic9x3} else {WindowType::Generic9x5}))};
        handler.behaviour.container_slots=count;
        for i in 0..count {handler.add_slot(Arc::new(LiveSlot{inventory:inventory.clone(),index:i,id:AtomicU8::new(0)}));}
        let own:Arc<dyn Inventory>=player_inventory.clone();
        // The lower player section retains vanilla ordering and native cursor handling.
        for i in (9..36).chain(0..9) {handler.add_slot(Arc::new(NormalSlot::new(own.clone(),i)));}
        Some(Arc::new(Mutex::new(handler)))
    }
}

/// Hidden opens never increment the world's public viewers. Only observers allowed
/// to see the opener get these private lid/sound updates; shared block state is untouched.
pub fn silent_container_effect(player:&Player,opened:bool) {
    use pumpkin_data::{Block, block_properties::{BarrelLikeProperties,ChestLikeProperties,ChestType}, HorizontalFacingExt, sound::{Sound,SoundCategory}};
    use pumpkin_protocol::java::client::play::{CBlockEvent,CBlockUpdate};
    let Some(position)=player.open_container_pos.load() else {return;};
    let world=player.world();let Some(server)=world.server.upgrade() else {return;};let state=world.get_block_state(&position);let block=state.id.to_block();
    let name=block.name;
    let sound=if name=="ender_chest" {if opened {Sound::BlockEnderChestOpen} else {Sound::BlockEnderChestClose}}
        else if name=="barrel" {if opened {Sound::BlockBarrelOpen} else {Sound::BlockBarrelClose}}
        else if name.ends_with("shulker_box") {if opened {Sound::BlockShulkerBoxOpen} else {Sound::BlockShulkerBoxClose}}
        else if name.ends_with("chest") {if opened {Sound::BlockChestOpen} else {Sound::BlockChestClose}}
        else {return;};
    let mut positions=vec![position];
    if name.ends_with("chest") && name!="ender_chest" {
        let props=ChestLikeProperties::from_state_id(state.id);
        let direction=match props.r#type {ChestType::Left=>Some(props.facing.rotate_clockwise()),ChestType::Right=>Some(props.facing.rotate_counter_clockwise()),_=>None};
        if let Some(direction)=direction {positions.push(position.offset(direction.to_block_direction().to_offset()));}
    }
    for viewer in world.players.load().iter() {
        if viewer.gameprofile.id!=player.gameprofile.id && !viewer.has_permission(&server,"arkstaff.vanish.see") {continue;}
        // A real public opener must never be visually closed by a hidden staff member.
        if !opened && world.players.load().iter().any(|p| p.gameprofile.id!=player.gameprofile.id && positions.iter().any(|pos|p.open_container_pos.load()==Some(*pos)) && viewer.can_see(&p.gameprofile.id)) {continue;}
        for pos in &positions {
            if name=="barrel" {
                let mut props=BarrelLikeProperties::from_state_id(state.id);props.open=opened;
                viewer.try_send_client_packet(&CBlockUpdate::new(*pos,i32::from(props.to_state_id(&Block::BARREL).as_u16()).into()));
            } else {
                let b=world.get_block_state(pos).id.to_block();
                viewer.try_send_client_packet(&CBlockEvent::new(*pos,1,u8::from(opened),i32::from(b.id.as_u16()).into()));
            }
        }
        viewer.play_sound(sound as u16,SoundCategory::Blocks,&position.to_centered_f64(),0.5,1.0,0);
    }
}
