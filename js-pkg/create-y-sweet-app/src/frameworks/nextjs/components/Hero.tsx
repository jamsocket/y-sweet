import Image from "next/image";

export function Hero() {
  return (
    <>
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
    </>
  );
}
