'use client'

import { STATUS_CONNECTED } from '@y-sweet/client'
import { useConnectionStatus, useHasLocalChanges } from '@y-sweet/react'

export default function StateIndicator() {
  let connectionStatus = useConnectionStatus()
  let hasLocalChanges = useHasLocalChanges()

  let statusColor = connectionStatus === STATUS_CONNECTED ? 'bg-green-500' : 'bg-red-500'
  let syncedColor = hasLocalChanges ? 'bg-red-500' : 'bg-green-500'

  return (
    <div className="mb-4">
      <div className="flex flex-row items-center text-xs space-x-1 w-fit bg-white rounded-md p-1 text-gray-500">
        <div>CONNECTED:</div>
        <div
          className={`w-3 h-3 rounded-full transition-colors ${statusColor}`}
          title={connectionStatus}
        ></div>
        <div>SYNCED:</div>
        <div
          className={`w-3 h-3 rounded-full transition-colors ${syncedColor}`}
          title={hasLocalChanges ? 'Unsynced local changes.' : 'No unsynced local changes.'}
        ></div>
      </div>
    </div>
  )
}
