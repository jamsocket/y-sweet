import './globals.css'
import type { Metadata } from 'next'

export const metadata: Metadata = {
  title: 'y-serv demos',
  description: 'Demos of the y-serv server.',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  )
}
