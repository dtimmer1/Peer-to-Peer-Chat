In this project, you will be working to implement some additional features in a P2P chat system.

The repository you have been given implements the basics, including the network architecture, as well as two sample user interfaces, one simple, one fancy.

# Usage

There are a few runnable programs in this repository.

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


# Grading

If you are able to demo, with a UI, a program that manages to propagate messages through the system, you will earn *70 points*.

If you are able to demo a peer that propagates messages through the system _and displays `Whisper` messages destined for the peer_, you will earn *10 points*.

Implementing the `Whisper` and `Deliver` commands (see the docs), will earn *30 points*.

Implementing an `Extension` command will earn *30 points*.

There are several places in the code noted with "points available."
If you would like to try addressing one of those, let me know and we'll determine how many points it will be worth.

There are also points available for doing something cool in general (there are a *lot* of opportunities here with the TUI in particular.)

I made an effort with documentation, but it is not as good as it could be.
Thus, there are points available for improving the documentation (talk to me about it before hand).

There are also many places where the code is messier than it should be, or where there might improvements possible via refactoring, etc.
The `fancy_tui` code in particular is nasty.
Again, points available but please talk to me first.
