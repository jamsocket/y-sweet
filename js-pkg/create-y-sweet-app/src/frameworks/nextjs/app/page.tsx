import { DocumentManager } from "@y-sweet/sdk";
import { YDocProvider } from "@y-sweet/react";

import { App } from "@/components/App";

const manager = new DocumentManager(
  process.env.CONNECTION_STRING || "ys://127.0.0.1:8080",
);

export default async function Home() {
  const docId = "my-doc-id";

  async function getClientToken() {
    "use server";
    // In a production app, this is where you'd authenticate the user
    // and check that they are authorized to access the doc.
    return await manager.getOrCreateDocAndToken(docId);
  }

  return (
    <YDocProvider docId={docId} authEndpoint={getClientToken}>
      <App />
    </YDocProvider>
  );
}
