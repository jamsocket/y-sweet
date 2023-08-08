'use client'

import { useState } from 'react'
import { LinkIcon } from '@heroicons/react/24/outline'

export default function CopyLink() {
  const [copied, setCopied] = useState(false)

  const copyLinkToClipboard = () => {
    const currentPageURL = window.location.href

    navigator.clipboard
      .writeText(currentPageURL)
      .then(() => {
        setCopied(true)
      })
      .catch((error) => {
        console.error('Failed to copy link: ', error)
      })
  }

  return (
    <div className="pt-6 flex text-left items-center text-neutral-500">
      <span className="pr-2">Share this document</span>
      <button
        className="flex text-sm items-center gap-1 px-2 py-1 rounded-lg bg-neutral-50 border-white border transition-all hover:bg-white"
        onClick={copyLinkToClipboard}
      >
        <LinkIcon className="h-3 w-3" />
        {copied ? 'Copied!' : 'Copy Link'}
      </button>
    </div>
  )
}
