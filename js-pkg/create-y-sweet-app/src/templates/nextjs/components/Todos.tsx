"use client";

import { useMap, useYSweetDebugUrl } from "@y-sweet/react";

export function Todos() {
  const debugUrl = useYSweetDebugUrl();
  const todos = useMap<boolean>("todos");

  return (
    <main className="flex flex-col p-6">
      <h2 className="font-bold text-lg mb-2">
        When you're working on your app…
      </h2>
      <ol className="list-decimal list flex flex-col gap-3 mb-12">
        <li className="grid grid-cols-[auto_1fr] grid-auto-flow-row gap-x-4 gap-y-1 items-center">
          <input
            className="peer"
            type="checkbox"
            checked={todos.get("sync") || false}
            onChange={(e) => todos.set("sync", e.target.checked)}
          />
          <p className="col-start-2 peer-checked:line-through">
            Open{" "}
            <a href={window.location.href} target="_blank">
              this page in a new window
            </a>
            .
          </p>
          <p className="col-start-2 text-xs text-gray-500">
            Every interactive element on this page syncs its state with all
            visitors.
          </p>
        </li>
        <li className="grid grid-cols-[auto_1fr] grid-auto-flow-row gap-x-4 gap-y-1 items-center">
          <input
            className="peer"
            type="checkbox"
            checked={todos.get("types") || false}
            onChange={(e) => todos.set("types", e.target.checked)}
          />
          <p className="col-start-2 peer-checked:line-through">
            Learn how to use our{" "}
            <a
              href="https://docs.jamsocket.com/y-sweet/reference/react"
              target="_blank"
            >
              Y-Sweet React library
            </a>
            .
          </p>
          <p className="col-start-2 text-xs text-gray-500">
            Y-Sweet's React hooks make it easy to use state that automatically
            syncs.
          </p>
        </li>
        <li className="grid grid-cols-[auto_1fr] grid-auto-flow-row gap-x-4 gap-y-1 items-center">
          <input
            className="peer"
            type="checkbox"
            checked={todos.get("debug") || false}
            onChange={(e) => todos.set("debug", e.target.checked)}
          />
          <p className="col-start-2 peer-checked:line-through">
            Inspect your document in the{" "}
            <a href={debugUrl} target="_blank">
              Y-Sweet debugger
            </a>
            .
          </p>
          <p className="col-start-2 text-xs text-gray-500">
            Y-Sweet lets you see the state of your document and everyone editing
            it in real time.
          </p>
        </li>
      </ol>

      <h2 className="font-bold text-lg mb-2">When you're ready to go live…</h2>
      <ol className="list-decimal list-inside flex flex-col gap-3">
        <li className="grid grid-cols-[auto_1fr] grid-auto-flow-row gap-x-4 gap-y-1 items-center">
          <input
            className="peer"
            type="checkbox"
            checked={todos.get("account") || false}
            onChange={(e) => todos.set("account", e.target.checked)}
          />
          <p className="col-start-2 peer-checked:line-through">
            Sign up for{" "}
            <a href="https://auth.jamsocket.com/en/signup" target="_blank">
              Jamsocket
            </a>
            .
          </p>
          <p className="col-start-2 text-xs text-gray-500">
            Jamsocket makes it easy to host Y-Sweet services.
          </p>
        </li>
        <li className="grid grid-cols-[auto_1fr] grid-auto-flow-row gap-x-4 gap-y-1 items-center">
          <input
            className="peer"
            type="checkbox"
            checked={todos.get("service") || false}
            onChange={(e) => todos.set("service", e.target.checked)}
          />
          <p className="col-start-2 peer-checked:line-through">
            Create a Y-Sweet service in the{" "}
            <a href="https://app.jamsocket.com" target="_blank">
              Jamsocket dashboard
            </a>
            .
          </p>
          <p className="col-start-2 text-xs text-gray-500">
            Y-Sweet is Jamsocket's service for building collaborative apps using
            Yjs.
          </p>
        </li>
        <li className="grid grid-cols-[auto_1fr] grid-auto-flow-row gap-x-4 gap-y-1 items-center">
          <input
            className="peer"
            type="checkbox"
            checked={todos.get("connstring") || false}
            onChange={(e) => todos.set("connstring", e.target.checked)}
          />
          <p className="col-start-2 peer-checked:line-through">
            Generate a connection string for your new Y-Sweet service.
          </p>
          <p className="col-start-2 text-xs text-gray-500">
            A connection string is like an API key that tells your Next.js app
            how to talk to Y-Sweet.
          </p>
        </li>
        <li className="grid grid-cols-[auto_1fr] grid-auto-flow-row gap-x-4 gap-y-1 items-center">
          <input
            className="peer"
            type="checkbox"
            checked={todos.get("env") || false}
            onChange={(e) => todos.set("env", e.target.checked)}
          />
          <p className="col-start-2 peer-checked:line-through">
            Add the connection string as the environment variable{" "}
            <code>CONNECTION_STRING</code>.
          </p>
          <p className="col-start-2 text-xs text-gray-500">
            Vercel has documentation on how to add an{" "}
            <a href="https://vercel.com/docs/projects/environment-variables">
              environment variable
            </a>{" "}
            to your project.
          </p>
        </li>
      </ol>
    </main>
  );
}
