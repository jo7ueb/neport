import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { App } from './App'
import { AuthProvider } from './context/AuthContext'
import { LocaleSyncProvider } from './context/LocaleSyncProvider'
import { SettingsProvider } from './context/SettingsContext'
import './i18n/config'
import './index.css'
import 'katex/dist/katex.min.css'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,
      refetchOnWindowFocus: false,
    },
  },
})

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <AuthProvider>
          <LocaleSyncProvider>
            <SettingsProvider>
              <App />
            </SettingsProvider>
          </LocaleSyncProvider>
        </AuthProvider>
      </BrowserRouter>
    </QueryClientProvider>
  </React.StrictMode>
)
