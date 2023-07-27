import Link from 'next/link'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  return (
    <div className="space-y-4 m-10">
      <h1 className="text-xl font-bold text-neutral-700 pb-4">Demos</h1>
      <div className='grid grid-cols-2 gap-4'>

        <Link href="/todos" className="text-neutral-700 hover:bg-red-100 transition-all p-10 border-2 border-black rounded-lg">
          To Do List
        </Link>


        <Link href="/editor" className="text-neutral-700 hover:bg-red-100 transition-all p-10 border-2 border-black rounded-lg">
          Code Editor
        </Link>

        <Link href="/text-editor" className="text-neutral-700 hover:bg-red-100 transition-all p-10 border-2 border-black rounded-lg">
          Text Editor
        </Link>

        <Link href="/color" className="text-neutral-700 hover:bg-red-100 transition-all p-10 border-2 border-black rounded-lg">
          Color Grid
        </Link>

      </div>
    </div>
  )
}
