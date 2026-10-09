import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// `appType: "spa"` so `/jobs/j-2041` opens directly rather than 404ing.
export default defineConfig({ plugins: [react(), tailwindcss()], appType: "spa" });
