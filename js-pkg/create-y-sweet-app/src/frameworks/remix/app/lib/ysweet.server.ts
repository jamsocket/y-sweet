import { DocumentManager } from "@y-sweet/sdk";

export const manager = new DocumentManager(
  process.env.CONNECTION_STRING || "ys://127.0.0.1:8080",
);
