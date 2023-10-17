# Example of implementations using Federated Subscriptions to not loose any events

## TLDR

If you're using Federated Subscriptions using the Apollo Router, during a connection lost you might loose events. To counter that risk of loosing events here is small examples of subgraph implementations that let's you implement your own logic to retrieve missing events based on an ID or a cursor. To achieve that you should design your subscription to take an optional ID/cursor that will let you send the last ID/cursor you received and it will replay all the missed events and still listen on new events.

## Subscription example

```graphql
type Subscription {
  """
  Listen to the list of all new players, if you specify the last ID you received, it will replay events starting from that ID
  """
  newPlayers(fromPlayerId: ID): Player!
}
```

The parameter `fromPlayerId` is our ID/cursor in this example. When provided it means that the subscription should replay all events sent after that ID/cursor and wait for new ones too.

## Implementation

This example is using an in memory database and broker, it will just filter the elements to replay based on their ID. If you want to replay events from a Kafka for example, it would be better to use a timestamp as a cursor instead of an ID. For example you can find several examples using a timestamp offset to replay events from a Kafka, like [this one for example](https://pranaysinghal.medium.com/replaying-kafka-messages-an-implementation-in-golang-e5c6867cf084).
