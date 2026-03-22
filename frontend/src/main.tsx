import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { FluentProvider, webDarkTheme } from "@fluentui/react-components";
import App from "./App.tsx";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <FluentProvider theme={webDarkTheme} style={{ minHeight: "100vh" }}>
      <App />
    </FluentProvider>
  </StrictMode>,
);
