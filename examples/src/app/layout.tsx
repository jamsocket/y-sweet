import './globals.css'
import type { Metadata } from 'next'
import Sidebar from '@/components/Sidebar'

export const metadata: Metadata = {
  title: 'y-sweet demos',
  description: 'Demos of y-sweet.',
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className="bg-[radial-gradient(at_bottom_left,_var(--tw-gradient-stops))] from-pink-50 to-pink-900">
        <Sidebar>{children}</Sidebar>
      </body>
    </html>
  )
}
