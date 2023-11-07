'use client'

import { useState } from 'react'
import { ArrowUpRightIcon, XMarkIcon } from '@heroicons/react/24/outline'

import Title from '@/components/Title'

export default function CopyLink() {

  const openLinkInNewTab = async () => {
    const currentPageURL = window.location.href

    try {
      window.open(currentPageURL, '_blank')
    } catch (err) {
      console.error('Failed to copy: ', err)
    }
  }
  //exclude from whiteboard, and toc
  // make it so you cant x it out, reduce vertical space

  return (
    <div className="mr-2 mb-2 text-left items-center text-neutral-500 border-2 border-yellow-200 rounded-lg bg-yellow-50 px-6 py-3">
      <div className="flex justify-between items-center pb-1">
        <Title>Collaborate on this document</Title>
        <div className="flex">
        <button
        className="text-sm flex items-center gap-1 px-3 py-1 rounded-lg bg-pink-950 text-white border transition-all "
        onClick={openLinkInNewTab}
      >
        Open link in a new tab
        <ArrowUpRightIcon className="h-4 w-4 font-bold" />
      </button>
        </div>
      </div>
      <div className="pr-2 w-full md:w-3/4">
        To simulate another user on this document, open the same link in a new tab. When you interact, you should see the user action happen
        in both windows.
      </div>

    </div>
  )
}
