'use client'

import { useState } from 'react'
import { ArrowUpRightIcon, XMarkIcon } from '@heroicons/react/24/outline'

import Title from '@/components/Title'

export default function CopyLink() {
  const [hideCallout, setHideCallout] = useState(false)

  const openLinkInNewTab = async () => {
    const currentPageURL = window.location.href

    try {
      window.open(currentPageURL, '_blank');
    } catch (err) {
      console.error('Failed to copy: ', err);
    }
  }

  if(hideCallout) {
    return <></>
  }

  return (
    <div className="mr-2 mb-2 text-left items-center text-neutral-500 border-2 border-yellow-200 rounded-lg bg-yellow-50 p-6 ">
      <div className="flex justify-between pb-2">
      <Title>How to collaborate on this document</Title>
      <button onClick={() => setHideCallout(true)}>
        <XMarkIcon className="h-5 w-5 hover:text-black"/>
      </button>
      </div>
      <span className="pr-2">To simulate another user appearing on this document, copy the link to this document and open it in a new window. When you interact on one screen, you should see the user action happens on another.
      </span>
      <button
        className="flex items-center gap-1 px-4 py-2 mt-4 rounded-lg bg-pink-950 text-white border transition-all "
        onClick={openLinkInNewTab}
      >
        Open link in a new tab
        <ArrowUpRightIcon className="h-5 w-5 " />
      </button>
    </div>
  )
}
