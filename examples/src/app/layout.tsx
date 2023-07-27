import './globals.css'
import type { Metadata } from 'next'
import Link from 'next/link'
import Logo from "../components/Logo"

export const metadata: Metadata = {
  title: 'y-sweet demos',
  description: 'Demos of y-sweet.',
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" >
      <body className="w-full bg-pink-900 flex h-screen">
      <div className=" bg-pink-950 w-60 text-white text-bold py-10 flex flex-col justify-between rounded-lg m-2">
        <div className="flex flex-col">
        <Link href="/" className="text-xl font-bold text-pink-100 pb-6 px-6">y-sweet</Link>
        <span className="px-6 text-xs tracking-wide text-pink-100 my-2">DEMOS</span>
        <Link href="/color" className="text-sm text-pink-100 hover:bg-pink-900 transition-all px-8 py-2">Color Grid</Link>
        <Link href="/todos" className="text-sm text-pink-100 hover:bg-pink-900 transition-all px-8 py-2">To-do List</Link>
        <Link href="/text-editor" className="text-sm text-pink-100 hover:bg-pink-900 transition-all px-8 py-2">Text Editor</Link>
        <Link href="/editor" className="text-sm text-pink-100 hover:bg-pink-900 transition-all px-8 py-2">Code Editor</Link>
        
        <span className="px-6 text-xs tracking-wide text-pink-100 mt-6 mb-2">LEARN</span>
        <Link href="https://y-sweet.dev" className="text-sm text-pink-100 hover:bg-pink-900 transition-all px-8 py-2">Docs</Link>
        </div>
        <div className="px-6">
        <Logo/>
        </div>
      </div>
        <div className="w-full bg-pink-50 mt-2 mr-2 rounded-lg mb-2">{children}</div>
      </body>
    </html>
  )
}
