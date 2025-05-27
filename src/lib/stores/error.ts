import { writable } from 'svelte/store';

export const ErrorBarType = {
    Error: 'error',
    Info: 'info',
    Success: 'success'
} as const;
export type ErrorBarType = typeof ErrorBarType[keyof typeof ErrorBarType];

// Use the union type for the interface
export interface ErrorBarState {
    message: string;
    visible: boolean;
    type: ErrorBarType; // "error" | "info" | "success"
}

export const errorBar = writable<ErrorBarState>({
    message: '',
    visible: false,
    type: ErrorBarType.Error
});

// Helper function to show error
export function showError(error_message: unknown, type: ErrorBarType = ErrorBarType.Error) {
    errorBar.set({ message: String(error_message), visible: true, type });
    setTimeout(() => errorBar.set({ message: '', visible: false, type }), 4000);
}