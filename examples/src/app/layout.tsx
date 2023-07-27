import './globals.css'
import type { Metadata } from 'next'

export const metadata: Metadata = {
  title: 'y-sweet demos',
  description: 'Demos of y-sweet.',
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>
        <div className="md:w-[800px] m-auto md:my-20">{children}</div>
      </body>
    </html>
  )
}
