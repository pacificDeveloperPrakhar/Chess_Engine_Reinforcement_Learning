
# How the Gateway, Manager Runtime, and Main Thread Work Together

## 1. The big picture
![Manager Gateway Main Thread Interaction](file:/Documentation_Resources/manager_gateway_main_thread_interaction.svg)

- The **main thread** starts everything and then waits.
- The **gateway** takes the client connections and turns the raw bytes into a request.
- The **manager runtime** does the real request handling.
- **Consistent hashing** decides which manager gets each request.

## 2. Main thread

- The main thread does not handle requests.
- It starts the gateway and the manager runtime.
- It keeps the process alive until the gateway finishes.

## 3. Gateway

- The gateway accepts each client connection.
- It reads the incoming bytes from the connection.
- It decodes those bytes into a structured request.
- Decoding happens in the gateway itself, not in the manager.
- The request types are defined in `/home/prakharv/Desktop/Chess/chess_engine/Server/src/Payload/Request.rs`.

## 4. Handing work to the manager

- After decoding, the gateway offloads the request to a manager task.
- The gateway does not wait for the manager to finish the work.
- The gateway only sends the decoded request and keeps handling other connections.

## 5. Consistent hashing

- The consistent hash struct is defined in the main thread. Everything else is triggered from there.
- It keeps two hash maps:
  1. **Connection map:** `u64` client ID → connection struct. Each entry represents one client connection.
  2. **Manager map:** `u64` ID → manager task. This maps to the manager task, not a server.
- Both kinds of ID are `u64` values, and the first bit (the most significant bit) tells them apart:
  - First bit `1` → connection ID.
  - First bit `0` → manager ID.
- Manager tasks can be spawned at any time while the program runs, and the spawn is triggered from the main thread.
- The consistent hash picks which manager gets each request, so the same client keeps going to the same manager.
- Adding or removing a manager moves only a few clients.

## 6. Manager runtime

- The manager runtime is a separate runtime from the gateway.
- It runs the manager tasks that handle all request logic.
- Each manager task takes a decoded request, does the work, and produces a reply.

## 7. Connection handling

- The gateway tracks every open connection.
- When a client disconnects, the gateway cleans up its connection.
- Requests from one connection are handled in order.