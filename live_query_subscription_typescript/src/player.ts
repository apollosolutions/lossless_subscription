import { ApolloServer } from "@apollo/server";
import { expressMiddleware } from "@apollo/server/express4";
import { ApolloServerPluginDrainHttpServer } from "@apollo/server/plugin/drainHttpServer";
import { buildSubgraphSchema } from "@apollo/subgraph";
import bodyParser from "body-parser";
import cors from "cors";
import express from "express";
import { readFileSync } from "fs";
import { PubSub } from "graphql-subscriptions";
import gql from "graphql-tag";
import { useServer } from "graphql-ws/lib/use/ws";
import { createServer } from "http";
import { WebSocketServer } from "ws";
const { json } = bodyParser;

function uuid() {
  return PLAYERS.length;
}

interface Player {
  id: string;
  name: string;
}

const pubsub = new PubSub();

const PLAYERS: Record<string, Player> = {};

const typeDefs = gql(readFileSync("./player.graphql", "utf-8"));

const resolvers = {
  Player: {
    __resolveReference(reference: Player) {
      return Object.values(PLAYERS).find(
        (player) => player.id === reference.id
      );
    },
  },

  Query: {
    players(_: undefined) {
      return PLAYERS;
    },
  },

  Mutation: {
    createPlayer(_: undefined, { userName }: { userName: string }): Player {
      const player: Player = {
        id: Object.keys(PLAYERS).length.toString(),
        name: userName,
      };
      PLAYERS[player.id] = player;
      pubsub.publish("CREATE_PLAYER", {
        newPlayers: player,
      });
      return player;
    },
  },

  Subscription: {
    newPlayers: {
      subscribe(_: undefined, { fromPlayerId }: { fromPlayerId: string }) {
        if (fromPlayerId != null) {
          const fromPlayerIdInt = Number.parseInt(fromPlayerId, 10);
          if (
            !isNaN(fromPlayerIdInt) &&
            isFinite(fromPlayerIdInt) &&
            fromPlayerIdInt < Object.keys(PLAYERS).length
          ) {
            return {
              async *[Symbol.asyncIterator]() {
                for (
                  let i = fromPlayerIdInt + 1;
                  i < Object.keys(PLAYERS).length;
                  i++
                ) {
                  console.log(PLAYERS[i.toString()]);
                  yield { newPlayers: PLAYERS[i.toString()] };
                }
                let iterator = {
                  [Symbol.asyncIterator]: () =>
                    pubsub.asyncIterator<Player>(["CREATE_PLAYER"]),
                };
                for await (const v of iterator) {
                  yield v;
                }
              },
            };
          } else {
            throw "bad value for 'fromPlayerId'";
          }
        }
        return pubsub.asyncIterator(["CREATE_PLAYER"]);
      },
    },
  },
};

const app = express();
const httpServer = createServer(app);

const schema = buildSubgraphSchema({ typeDefs, resolvers });
const wsServer = new WebSocketServer({
  server: httpServer,
  path: "/ws",
});
const serverCleanup = useServer({ schema }, wsServer);

const server = new ApolloServer({
  schema,
  plugins: [
    ApolloServerPluginDrainHttpServer({ httpServer }),
    {
      async serverWillStart() {
        return {
          async drainServer() {
            await serverCleanup.dispose();
          },
        };
      },
    },
  ],
});

await server.start();
app.use("/", cors(), json(), expressMiddleware(server));

const PORT = process.env.PORT || 4006;
httpServer.listen(PORT, () => {
  console.log(`🚀 Player subgraph ready at http://localhost:${PORT}/`);
});
