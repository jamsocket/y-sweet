import { ActionFunctionArgs } from "@remix-run/node";
import { type MetaFunction } from "@remix-run/react";

import { manager } from "~/lib/ysweet.server";

export const meta: MetaFunction = () => [{ title: "Y-Sweet + Remix" }];

export async function action({ request }: ActionFunctionArgs) {
  const { docId } = await request.json();
  const token = await manager.getOrCreateDocAndToken(docId);

  return Response.json(token);
}
