import React from 'react'
import ReactDOM from 'react-dom/client'
import { App } from './App'
import '../styles/prototype.css'
import '../styles/app.css'
import '../styles/controls.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)
