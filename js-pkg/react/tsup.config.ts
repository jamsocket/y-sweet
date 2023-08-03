import { defineConfig } from "tsup"

export default defineConfig({
  entry: ["src/main.tsx"],
  dts: true,
  splitting: true,
  clean: true,
  target: "es2020",
  format: ["esm", "cjs"],
  sourcemap: true,
})
