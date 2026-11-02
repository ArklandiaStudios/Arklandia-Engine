#[allow(clippy::wildcard_imports)]
use super::*;

impl JavaClient {
    pub fn handle_edit_book(&self, player: &Player, packet: &SEditBook<'_>) {
        let held_stack = player.inventory().held_item();
        if held_stack.item.id != Item::WRITABLE_BOOK.id {
            return;
        }

        let mut pages: Vec<String> = packet.pages.iter().map(|p| (*p).to_string()).collect();
        let mut title = packet.title.map(std::string::ToString::to_string);
        let signing = title.is_some();
        let slot = player.inventory().get_selected_slot() as u32;

        if let Some(player_arc) = player.world().get_player_by_uuid(player.gameprofile.id)
            && let Some(server) = player.world().server.upgrade()
        {
            let mut event =
                crate::plugin::api::events::player::player_edit_book::PlayerEditBookEvent {
                    player: player_arc,
                    slot,
                    pages: pages.clone(),
                    title: title.clone(),
                    signing,
                    cancelled: false,
                };
            server.plugin_manager.fire_blocking(&server, &mut event);
            if event.cancelled {
                return;
            }
            pages = event.pages;
            title = event.title;
        }

        if let Some(title) = title {
            let mut written_book = ItemStack::new(1, &Item::WRITTEN_BOOK);
            let content = WrittenBookContentImpl {
                title: pumpkin_data::data_component_impl::Filterable::new(title),
                author: player.gameprofile.name.clone(),
                pages: pages.into_iter().map(|page| pumpkin_data::data_component_impl::Filterable::new(pumpkin_util::text::TextComponent::text(page))).collect(),
                generation: 0,
                resolved: true,
            };
            written_book
                .patch
                .push((DataComponent::WrittenBookContent, Some(content.to_dyn())));
            player.inventory().set_held_item(written_book);
        } else {
            let mut writable_book = held_stack;
            let content = WritableBookContentImpl { pages: pages.into_iter().map(pumpkin_data::data_component_impl::Filterable::new).collect() };
            writable_book
                .patch
                .retain(|(component, _)| *component != DataComponent::WritableBookContent);
            writable_book
                .patch
                .push((DataComponent::WritableBookContent, Some(content.to_dyn())));
            player.inventory().set_held_item(writable_book);
        }
    }
}
