import { defineConfig } from "tsup"

export default defineConfig({
  entry: ["src/main.ts"],
  dts: true,
  splitting: true,
  clean: true,
  target: "es2020",
  format: ["esm", "cjs"],
  sourcemap: true,
})
