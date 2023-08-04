'use client'
import { useState } from 'react'

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
    <div className="pt-6 flex text-left items-center">
      <span className="text-neutral-400 pr-2">Share this document</span>
      <button
        className="w-24 py-1 rounded-lg bg-neutral-50 border-white border text-neutral-400 transition-all hover:bg-white"
        onClick={copyLinkToClipboard}
      >
        {copied ? 'Copied!' : 'Copy Link'}
      </button>
    </div>
  )
}
