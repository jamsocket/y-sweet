"use client"
import './globals.css'
// import type { Metadata } from 'next'
import Sidebar from '@/components/Sidebar'
import CopyLink from '@/components/CopyLink'
import { useEffect, useState } from 'react'

// export const metadata: Metadata = {
//   title: 'y-sweet demos',
//   description: 'Demos of y-sweet.',
// }

export default function RootLayout({ children }: { children: React.ReactNode }) {
  const [showCopyLink, setShowCopyLink] = useState(true)

  useEffect(() => {
    const path = window.location.pathname
    if(path === '/' || path === '/whiteboard') {
      setShowCopyLink(false)
    }
  }, [])

  return (
    <html lang="en">
      <body className="bg-[radial-gradient(at_bottom_left,_var(--tw-gradient-stops))] from-pink-50 to-pink-900">
        <Sidebar>
          <CopyLink hidden={!showCopyLink}/>
          <div className="absolute w-full h-full top-0 pt-14 lg:relative lg:w-auto lg:pt-0 bg-[radial-gradient(at_bottom_left,_var(--tw-gradient-stops))] from-white/90 via-pink-50/90 to-pink-100/90 sm:mr-2 sm:rounded-lg">
            {children}
          </div>
        </Sidebar>
      </body>
    </html>
  )
}
