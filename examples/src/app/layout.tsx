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
        <Sidebar>
          <div className="absolute w-full h-full top-0 pt-14 lg:relative lg:w-auto lg:pt-0 bg-[radial-gradient(at_bottom_left,_var(--tw-gradient-stops))] from-white/90 via-pink-50/90 to-pink-100/90 sm:mr-2 sm:rounded-lg">
            {children}
          </div>
        </Sidebar>
      </body>
    </html>
  )
}
