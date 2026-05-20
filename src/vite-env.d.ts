/// <reference types="vite/client" />

declare namespace React {
  interface HTMLAttributes<T> {
    "data-tauri-drag-region"?: boolean;
  }
}
