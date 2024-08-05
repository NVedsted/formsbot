use serenity::all::{GenericId, RoleId, UserId};
use crate::ApplicationContext;
use crate::state::SerializableMention;

pub trait ApplicationContextExt {
    fn to_mentionable(&self, id: GenericId) -> Option<SerializableMention>;
}

impl<'a> ApplicationContextExt for ApplicationContext<'a> {
    fn to_mentionable(&self, id: GenericId) -> Option<SerializableMention> {
        if let Some(user) = self.cache().user(UserId::new(id.get())) {
            return Some(SerializableMention::User(user.id));
        }

        self.guild().and_then(|g| g.roles.get(&RoleId::new(id.get())).map(|r| SerializableMention::Role(r.id)))
    }
}
