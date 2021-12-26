This is my final project for CS451. The goal was to implement a targeted message command in a peer-to-peer chat system we had been building up over the course of the semester.

# Usage

## Tracker

You can start up a tracker by running `cargo run -p libb2b --bin tracker`.
You can pass command line arguments in when using cargo: `cargo run -p libb2b --bin tracker -- --host 0.0.0.0 --port 3001` (note the `--` after `--bin tracker`).

The tracker takes an ip address and port to listen on.
If you set the ip address to 0.0.0.0 it will listen on whatever ip addresses the machine responds to.

## Client

You can start up a simple client by running `cargo run -p bing2bing-tui`.
The tui takes several command line arguments, and they are not entirely intuitive.

1. `--host` this is the ip address that "your" peer will listen on.
Note that you *cannot* set this to be 0.0.0.0 because this value is directly used when sending out protocol messages. However, this could be automated (e.g., as points that you could earn).

2. `--port` the port that your peer will listen on. Note that this must be unique for whatever machine you are running it on!

3. `--tracker-host` the ip address of a tracker to connect to and boostrap yourself into the network.

4. `--tracker-port` the port of the tracker to connect to.

5. `--name` the name that this peer will go by.

6. `--simple` if set, you will start up a simple client that only responds to `/say` and `/quit` input. If you don't set it, then a fancier UI will be used. For the fancy TUI, pressing `e` will put you into edit mode, pressing `esc` will bring you out of edit mode, typing `q` will quit, and pressing `l` or `h` will move between the two tabs.

# Post-Mortem

My process for implementing the Whisper command followed a few steps:

1) Allow the UI to recognize messages beginning with /whisper.
2) Initially I had Whisper commands implemented just like Say commands with a destination tacked on, as an intermediate step. The next step was figuring out how to route them.
3) After writing up Dijkstra's, I had to figure out where to call it. My options were between calling Dijkstra's at every step, and calling it once and forwarding the route along to each successive peer. I ultimately settled on the second one, although like we talked about in the demo that did break the protocol.
4) I implemented a basic send_to_peer() function in the PeerMap to send frames to one specific peer. Since this doesn't affect the messages sent and received, I don't think this one breaks the protocol.
5) With routing and sending to one specific peer implemented, I had a successful Whisper command (besides protocol breaking). My stretch goal after that was to try and do latency testing, but finals week decided otherwise :(

The project taught me a lot about asynchronous programming as well as Rust. Interestingly, I found that most of my time was spent understanding the control flow of the system. That's probably the case with most systems, though, as there are a lot of moving parts to keep track of. Learning how to do that effectively might have been my biggest takeaway from the project.
