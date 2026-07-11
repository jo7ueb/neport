/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_GIT_COMMIT?: string
  readonly VITE_GIT_DIRTY?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
