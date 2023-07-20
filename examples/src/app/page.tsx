import Link from 'next/link'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  return (
    <div className="space-y-4">
      <h1 className="text-xl font-bold">Home</h1>
      <div><Link href="/todos" className="text-blue-500 underline">To Do List</Link></div>
    </div>
  )
}
