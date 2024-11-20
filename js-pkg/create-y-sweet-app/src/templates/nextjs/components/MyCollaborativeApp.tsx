"use client";

import Image from "next/image";
import {
  useMap,
  useYSweetDebugUrl,
  usePresence,
  usePresenceSetter,
} from "@y-sweet/react";
import { useEffect, useState } from "react";
import { randomColor } from "@/lib/colors";

export function MyCollaborativeApp() {
  const debugUrl = useYSweetDebugUrl();

  const todos = useMap<boolean>("todos");

  const setPresence = usePresenceSetter();
  const presence = usePresence();

  const [self, setSelf] = useState(() => ({ color: randomColor() }));
  useEffect(() => setPresence(self), [setPresence, self]);

  const others = Array.from(presence.entries());

  return (
    <div
      className="min-h-screen flex flex-col justify-between max-w-[60rem] mx-auto before:content-[''] before:block before:absolute before:top-0 before:left-0 before:right-0 before:border-t-4 before:border-[var(--color,#fc5c86)]"
      style={{ "--color": self.color } as React.CSSProperties}
    >
      <header className="flex flex-col gap-8 p-12">
        <div className="flex justify-center items-center gap-6">
          <Image src="/y-sweet.svg" alt="Y-Sweet" width="116" height="38" />
          +
          <Image src="/next.svg" alt="Next.js" width="98" height="20" />
        </div>

        <div className="max-w-2xl mx-auto flex flex-col gap-4">
          <h1 className="text-center text-3xl text-balance ">
            Y-Sweet is an open-source Yjs server by{" "}
            <strong>
              <a href="https://jamsocket.com" target="_blank">
                Jamsocket
              </a>
            </strong>{" "}
            for building <strong>collaborative apps</strong>.
          </h1>
          <p className="text-lg text-center text-gray-500 text-balance">
            Everything on this website automatically syncs!
            <br />
            Open multiple windows to see more bubbles appear.
          </p>
        </div>

        <div className="inline-block relative mx-auto pb-[102px]">
          <ul className="flex justify-center items-center gap-2">
            <li
              className="flex rounded-full border border-dashed p-0.5"
              style={{ borderColor: self.color }}
            >
              <label
                className="block appearance-none w-6 h-6 rounded-full"
                style={{ backgroundColor: self.color }}
              >
                <input
                  className="invisible"
                  type="color"
                  value={self.color}
                  onChange={(e) => setSelf({ ...self, color: e.target.value })}
                />
              </label>
            </li>
            {others.map(([id, peer]) => (
              <li
                key={id}
                className="rounded-full w-6 h-6"
                style={{ backgroundColor: peer.color }}
              ></li>
            ))}
          </ul>
          <Image
            className="absolute max-w-none mt-[4px] -left-6"
            src="/tip.svg"
            alt=""
            width="116"
            height="98"
          />
        </div>
      </header>

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
              Y-Sweet lets you see the state of your document and everyone
              editing it in real time.
            </p>
          </li>
        </ol>

        <h2 className="font-bold text-lg mb-2">
          When you're ready to go live…
        </h2>
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
              Y-Sweet is Jamsocket's service for building collaborative apps
              using Yjs.
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
              <code>Y_SWEET_CONNECTION_STRING</code>.
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

      <footer className="p-8">
        <ul className="flex justify-center gap-8">
          <li>
            <a className="text-xs text-gray-500" href="https://jamsocket.com">
              <span>Jamsocket</span>
            </a>
          </li>
          <li>
            <a
              className="text-xs text-gray-500"
              href="https://docs.jamsocket.com/y-sweet/"
            >
              <span>Y-Sweet</span>
            </a>
          </li>
          <li>
            <a className="text-xs text-gray-500" href="https://docs.yjs.dev">
              <span>Yjs</span>
            </a>
          </li>
          <li>
            <a
              className="text-xs text-gray-500"
              href="https://github.com/jamsocket/y-sweet"
            >
              <span>Source</span>
            </a>
          </li>
          <li>
            <a
              className="text-xs text-gray-500"
              href="https://discord.gg/RFrDbMVKxv"
            >
              <span>Get Help</span>
            </a>
          </li>
        </ul>
      </footer>
    </div>
  );
}
