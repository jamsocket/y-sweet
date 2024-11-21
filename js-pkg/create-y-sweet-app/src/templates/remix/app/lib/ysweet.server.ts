import { DocumentManager } from "@y-sweet/sdk";

export const manager = new DocumentManager(
  process.env.Y_SWEET_CONNECTION_STRING!,
);
