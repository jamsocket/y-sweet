import './globals.css'
import type { Metadata } from 'next'

export const metadata: Metadata = {
  title: 'Y-Sweet Debugger',
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className="bg-slate-800 text-gray-100">
        <div className="p-8">
          <h1 className="text-xl font-bold mb-5">y-sweet Debugger</h1>
          {children}
        </div>
      </body>
    </html>
  )
}
