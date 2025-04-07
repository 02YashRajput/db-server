# Database Server

A simple in-memory key-value database server with authentication support and TTL (Time-To-Live) functionality.

## Features

- **In-memory storage**: Fast key-value operations
- **Authentication**: Optional username/password protection for databases
- **TTL support**: Keys can expire after specified durations (seconds, minutes, days)
- **Multi-database support**: Create and switch between multiple databases
- **Cleaner thread**: Automatic removal of expired keys
- **TCP interface**: Network-accessible server
- **File-Storage**: File storage for persistent memory 

## Prerequisites

- Rust toolchain (install via [rustup](https://rustup.rs/))
- For development: `tokio` async runtime

## Installation

1. Clone the repository
2. Run the project

```bash
cargo run
```
Run with default port (4000):
Server running on 127.0.0.1:4000
```bash
cargo run <port>
```
Server running on 127.0.0.1:[port]


## Usage
### Starting the Server
Run with default port (4000):
```bash
cargo run
```
Run with custom port:
```bash
cargo run <port>
```

### Client Commands
Use with the [companion client](https://github.com/02YashRajput/db-cli) or any TCP client.

#### Database Operations:
+ `create <dbname>` - Create a new database (optionally with authentication)

+ `use <dbname>` - Select a database (authenticate if required)


#### Key-Value Operations:
+ `SET("key","value",["ttl"])` - Store a value (optional TTL: "5s", "10m", "1d")

+ `GET("key")` - Retrieve a value

+ `DEL("key")` - Delete a key

#### Session:
+ `exit` - Disconnect from server

## Architecture
### Components
1. Main Server (main.rs):

    + Handles TCP connections

    + Manages client sessions

    + Routes commands to appropriate handlers

2. Database Core (db.rs):

    + Implements database storage

    + Manages authentication

    + Handles TTL for keys

3. Parser (parser.rs):

    + Processes client commands

    + Validates syntax

    + Executes operations

4. Cleaner (cleaner.rs):

    + Background thread for removing expired keys

    + Periodic database maintenance
     
    + Periodic file maintienance 

5. Logger (logger.rs):

    + Logging functionality (to be implemented)

## Configuration
The server supports:

+ Custom port via command line argument

+ Optional authentication per database

+ Automatic key expiration

## Performance
+ Uses Rust's HashMap for fast lookups

+ Arc<Mutex> for thread-safe concurrent access

+ Tokio for async I/O operations

## Limitations
+ Simple TCP protocol (no encryption)

## Related Projects
[db-client](https://github.com/02YashRajput/db-cli)  - Companion client application