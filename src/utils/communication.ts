import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";

export async function callAndReceiveAsync<T>(
    command: string,
    message: { decode: (bytes: Uint8Array) => T },
    args?: Record<string, unknown>
): Promise<T> {
    const raw = await invoke(command, args);
    let bytes: Uint8Array;
    if (raw instanceof Uint8Array) {
        bytes = raw;
    } else if (Array.isArray(raw)) {
        bytes = new Uint8Array(raw);
    } else {
        throw new Error("Unexpected response type from backend: " + typeof raw);
    }
    return message.decode(bytes);
}

/**
 * Listen for a backend event from Tauri, decode the protobuf payload, and call the handler.
 * @param eventName - The event name emitted by the backend (Rust).
 * @param message - The protobuf message class/object (e.g., BackendErrorOccurred).
 * @param handler - Function to call with the decoded message.
 */
export function listenBackendEvent<T>(
    message: { $type: string, decode: (bytes: Uint8Array) => T },
    handler: (msg: T) => void
) {
    listen<Uint8Array>(message.$type.split(".").at(-1)!, (event) => {
        try {
            // Convert payload to Uint8Array if necessary
            let bytes: Uint8Array;
            if (event.payload instanceof Uint8Array) {
                bytes = event.payload;
            } else if (Array.isArray(event.payload)) {
                bytes = new Uint8Array(event.payload);
            } else {
                throw new Error("Unexpected payload type: " + typeof event.payload);
            }
            const msg = message.decode(bytes);
            handler(msg);
        } catch (e) {
            console.error(`Failed to decode event ${message.$type.split(".").at(-1)!}:`, e);
        }
    });
}