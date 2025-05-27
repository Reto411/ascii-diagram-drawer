import {invoke} from "@tauri-apps/api/core";

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