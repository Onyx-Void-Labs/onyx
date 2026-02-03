import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// PREVENT RIGHT CLICK (CONTEXT MENU)
document.addEventListener('contextmenu', (event) => {
  event.preventDefault();
  // "preventDefault" tells the browser: "I'll handle this. You do nothing."
})

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);