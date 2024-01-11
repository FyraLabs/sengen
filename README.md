# Sengen :boom: :wave:

Sengen is a multi-protocol, multi-platform chat server written in Rust, designed for compatibility with multiple chat protocols/clients, legacy and modern.

It's designed to be an alternative to [escargot.chat](https://escargot.chat), which is a revival of the MSN Messenger protocol. Made to be modern and blazing fast :rocket::crab:

## Features

Sengen is currently in very early development (in the experimental phase), and not even close to production ready. However, the following features are planned:

- [ ] IRC support
- [ ] XMPP support
- [ ] Matrix support
- [ ] MSNP support
- [ ] AIM support
- [ ] ICQ support
- [ ] Yahoo! Messenger support
- [ ] A web chat client

Sengen will be designed to be modular, for stability and ease of development.

Sengen has a main server, which handles all the connections, and then a series of microservices which handle the actual protocol logic. This means that if one protocol shim crashes, the rest of the server will be unaffected.

Right now we are working on the main server, and then we will move on to the chat protocol shims.

## Name

> Sengen (宣言) is a Japanese word meaning "declaration" or "proclamation". This project is actually named after the song [Goodbye Declaration](https://youtu.be/dHXC_ahjtEE) by Chinozo.
