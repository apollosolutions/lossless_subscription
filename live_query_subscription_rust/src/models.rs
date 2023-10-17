use async_graphql::{Context, Object, SimpleObject, Subscription, ID};
use futures_util::{stream, Stream, StreamExt};
use tokio::sync::{
    broadcast::{self},
    RwLock,
};
use tokio_stream::wrappers::BroadcastStream;

#[derive(Default)]
pub(crate) struct InMemoryDb {
    players: RwLock<Vec<Player>>,
}

impl InMemoryDb {
    pub(crate) async fn get_player(&self, player_id: usize) -> Option<Player> {
        self.players.read().await.get(player_id).cloned()
    }

    pub(crate) async fn get_players(&self) -> Vec<Player> {
        self.players.read().await.clone()
    }

    /// Get players created after that specific player_id
    pub(crate) async fn get_players_after(&self, player_id: usize) -> Vec<Player> {
        self.players
            .read()
            .await
            .iter()
            .skip(player_id + 1)
            .cloned()
            .collect()
    }

    pub(crate) async fn create_player(&self, username: String) -> Option<Player> {
        let mut players = self.players.write().await;
        if players.iter().any(|p| p.name == username) {
            // Conflict username already took
            return None;
        }

        let new_player = Player {
            id: ID::from(players.len().to_string()),
            name: username,
        };

        players.push(new_player.clone());

        Some(new_player)
    }
}

pub(crate) struct InMemoryBroker {
    players: broadcast::Sender<Player>,
}

impl Default for InMemoryBroker {
    fn default() -> Self {
        let (tx, _) = broadcast::channel(2);
        Self { players: tx }
    }
}

impl InMemoryBroker {
    pub(crate) async fn subscribe_new_players(&self) -> impl Stream<Item = Player> {
        let players_stream = BroadcastStream::new(self.players.subscribe())
            .filter_map(|e| async move { e.ok() })
            .boxed();

        players_stream
    }

    pub(crate) async fn new_player(&self, player: Player) {
        let _err = self.players.send(player);
    }
}

pub(crate) struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn players<'ctx>(&self, ctx: &Context<'ctx>) -> Vec<Player> {
        let in_memory_db: &InMemoryDb = ctx.data_unchecked();
        in_memory_db.get_players().await
    }

    #[graphql(entity)]
    async fn find_player_by_id_and_quiz_id<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        id: ID,
    ) -> Option<Player> {
        let in_memory_db: &InMemoryDb = ctx.data_unchecked();

        in_memory_db
            .get_player(id.as_str().parse::<usize>().ok()?)
            .await
    }
}

pub(crate) struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    async fn new_players<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        from_player_id: Option<ID>,
    ) -> async_graphql::Result<impl Stream<Item = Player>> {
        let in_memory_broker: &InMemoryBroker = ctx.data_unchecked();

        let player_stream = in_memory_broker.subscribe_new_players().await;
        let player_stream = match from_player_id {
            Some(from_player_id) => {
                let from_player_id = from_player_id
                    .as_str()
                    .parse::<usize>()
                    .map_err(|_| "cannot convert fromPlayerId to a number")?;
                let in_memory_db: &InMemoryDb = ctx.data_unchecked();
                let missed_players = in_memory_db.get_players_after(from_player_id).await;
                stream::iter(missed_players).chain(player_stream).boxed()
            }
            None => player_stream.boxed(),
        };

        Ok(player_stream)
    }
}

pub(crate) struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_player<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        user_name: String,
    ) -> async_graphql::Result<Player> {
        let in_memory_db: &InMemoryDb = ctx.data_unchecked();

        let new_player = in_memory_db
            .create_player(user_name)
            .await
            .ok_or_else(|| async_graphql::Error::new("cannot create a player"))?;

        let in_memory_broker: &InMemoryBroker = ctx.data_unchecked();
        in_memory_broker.new_player(new_player.clone()).await;

        Ok(new_player)
    }
}

#[derive(Clone, SimpleObject, Debug)]
pub(crate) struct Player {
    pub(crate) id: ID,
    pub(crate) name: String,
}
