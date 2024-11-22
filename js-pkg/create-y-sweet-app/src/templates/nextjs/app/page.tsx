import { DocumentManager } from "@y-sweet/sdk";
import { YDocProvider } from "@y-sweet/react";

import { MyCollaborativeApp } from "@/components/MyCollaborativeApp";

const manager = new DocumentManager(process.env.CONNECTION_STRING!);

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
      <MyCollaborativeApp />
    </YDocProvider>
  );
}
