import { type MetaFunction } from "@remix-run/react";
import { YDocProvider } from "@y-sweet/react";

import { App } from "~/components/App";

export const meta: MetaFunction = () => [{ title: "Y-Sweet + Remix" }];

export function clientLoader() {
  return Response.json({});
}

export function HydrateFallback() {
  return <div />;
}

export default function Home() {
  const docId = "my-doc-id";

  return (
    <YDocProvider docId={docId} authEndpoint="/auth">
      <App />
    </YDocProvider>
  );
}
