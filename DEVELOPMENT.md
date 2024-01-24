# Development
Here follow the development of the project described step by step.

## Runtime
The first step was finding the most suitable runtime to realize this project. The choice fell on one of the most popular async runtime existing in Rust, [`tokio`](https://tokio.rs). Once followed the introductive tutorial, I was convinced that it was the best choice among the others.

## First Implementation
[*go to commit*](https://github.com/lorenzo-dominici/commnode/tree/85013fe959c9613204cacc8ab4f67aed070d9b3c)
In the first phase of implementation I defined the main components of the dispatcher system, but their structures were still incomplete.
The subscription cancellation were still on-demand, there was no distinction between events and commands and the topics themselves were still not defined, but the main logic behind the event-dispatching were already implemented.

## Better Structure
[*go to commit*](https://github.com/lorenzo-dominici/commnode/tree/fcdffe0ba976446a980b736fd66ece3b76845de5)
Here the more general concept of interest, based on [`regex`](https://docs.rs/regex/latest/regex/), were introduced and events were separated from commands to the dispatcher.

## Serialization
The responibility of serialization and deserialization of data structures, fundamental for this kind of project, fell on the crate [`serde`](https://serde.rs), which I already had familiarity with, thanks to other minor projects. My preference aside, [`serde`](https://serde.rs) is the most used crate for serialization in Rust.

## Framing
[*go to commit*](https://github.com/lorenzo-dominici/commnode/tree/0bb98391853caa47cbe4c276567ec8f9eec1a516)
The need of the implementation of framing techniques arose when I started to think about the parsing needed before deserialization when a serialized event would pass through a TCP stream. For this purpose, I choose to adopt a simple but effective *length delimited codec*, offered by the [`tokio-util`](https://docs.rs/tokio-util/latest/tokio_util) crate. The framing method implemented was just a first try, it has been slightly improved in following commits, but without changing its logic significantly.

## TCP
[*go to commit*](https://github.com/lorenzo-dominici/commnode/tree/74e7f1038fee2c5d30b394c97033a6a26008a26f)
With this and part of the next commit, the implementation of the methods needed for an easy use of TCP communication were done. The methods were designed to handle sending and receiving events through framed streams and local channels. The design adopted is meant to be shared by other protocols implementation.

## Cancellation
[*go to commit*](https://github.com/lorenzo-dominici/commnode/tree/398d9447504f19e20e443bf3179fde3330b453c2)
Thanks to the cancellation utility of [`tokio-util`](https://docs.rs/tokio-util/latest/tokio_util), it has been possible to easily implement techniques for graceful shutdown of the system, preventing any task to keep running in background, and allowing the correct unwinding of the program.
As part of the same commit, it has been introduced the automatic deactivation of subscription, based on the activity of the channel.

## UDP
[*go to commit*](https://github.com/lorenzo-dominici/commnode/tree/621b76ccb8495e1695b20d11f58da9cd22addbe2)
Following the implementation of the TCP methods, and using the [`udp-stream`](https://docs.rs/udp-stream/latest/udp_stream) crate, the methods to handle UDP communications have been implemented.

## Tests
[*go to commit*](https://github.com/lorenzo-dominici/commnode/tree/7e7bf091444c672f648f7866910bb48bd3cde8a7)
To test the functionalities implemented, some tests have been implemented, following the structure that `cargo` needs to execute them properly.
Further tests have been implemented in later commits to test every major component of the program.

## Config Format
Where some projects use `JSON`, others use `YAML` or `XML`. The choice of the configuration format fell on `TOML`, partially because of its simplicity, and also because it is the one chosen by Rust for its compiler.

## Configuration
[*go to commit*](https://github.com/lorenzo-dominici/commnode/tree/5af2605ee50e4b4f1a8ad902758edd9b99a4167d)
Here starts the implementation of the configuration feature, where its fundamental method is the one that parse the `TOML` file into a congruent data structure.
In the following commits there have been implemented methods that, given the configuration data parsed, initialize tasks with the roles of senders and receivers.
In this way, the program offers a fast and simple way to configure the access points (receivers) of the commnode, and the connections (senders) to other nodes.
The main advantage of this practice is that the Rust program does not need to be modified, and hence re-compiled, if the user wants to change the commnode connecitons.

## Bridge
[*go to commit*](https://github.com/lorenzo-dominici/commnode/tree/f2c06547ce7bcf2d95477122723383b728849ea4)
From this commit begins the implementation of the features realized so far. The bridge program is one possible implementation of this architecture, and its role is to act as a proxy between the commnode and a third program, that in this case is the python scripts responsible for the federated learning.

## Discovery
[*go to commit*](https://github.com/lorenzo-dominici/commnode/tree/fc0da6c31746f5fa6308226f0a4752f365eae5d7)
As part of the configuration of external communication channels, it has been implemented an advertisement mechanism, so that the first message of each sender contains the data needed to create a connection with the configured receivers.
This allows an easy setup of "client-like" commnodes.

## Python Bridge
[*go to commmit*](https://github.com/lorenzo-dominici/commnode/tree/3ca7eb7859a4af8b7f8424244448ecd63ad1be91)
For the connection with the Rust bridge, a complementary bridge in Python has been implemented. It offers just the features needed to send and receive serialized data to and from the Rust bridge.

## Federated Learning
[*go to commit*](https://github.com/lorenzo-dominici/commnode/tree/21cd8e8a91e5553911837b8e1d6d0bd2c10730c3)
For the realization of the federated learning system, two more Python scripts have been implemented, one with the function of local model trainer, and the other with the function of model aggregator, according to the logic of federated learning.

## Experiments & Trials
Some experiment has been conducted to test the functioning of the federated learning system based on the commnode communications. Further details of these experiments can be found [here](/EXPERIMENT.md).

## Tracking & Logging
[*go to commit*](https://github.com/lorenzo-dominici/commnode/tree/d7dea6bf003fbe8721531fcda02366bdb9f6430e)
For a better understanding of the funcitoning of the system, further logging has been implemented to track every communication in input and in output, every subscription and every event dispatching.