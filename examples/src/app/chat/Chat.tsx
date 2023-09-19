'use client'
import { useArray, useYDoc } from '@y-sweet/react'
import { usePresence } from '@y-sweet/react'
import { usePresenceSetter } from '@y-sweet/react'
import { useState, useEffect } from 'react'

import * as Y from 'yjs'

type Presence = {
  username: string
  isTyping: boolean
}

export default function Chat() {
  const doc = useYDoc()
  const messages = useArray<Y.Map<any>>('messages')
  const [message, setMessage] = useState('')
  const presence = usePresence<Presence>({
    includeSelf: true
  })
  const setPresence = usePresenceSetter<Presence>()

  useEffect(() => {
    setPresence({
      username: `user${doc.clientID}`,
      isTyping: false,
    })
    setPresence({
      username: 'GPT',
      isTyping: false,
    })
  }, [doc.clientID, setPresence])

  const getOpenAIResponse = async (message: string) => {
    setMessage('')
    const res = await fetch('/api', {
      method: 'POST',
      body: JSON.stringify({
        message,
      }),
      headers: {
        'Content-type': 'application/json',
      },
    })

    if (!res.ok || !res.body) {
      return
    }
    const reader = res.body.getReader()
    const decoder = new TextDecoder()
    let done = false
    //TODO FIX THIS
    let chunks: string[] = []

    while (!done) {
      const { value, done: doneReading } = await reader.read()
      done = doneReading
      const chunkValue = decoder.decode(value)
      chunks = [...chunks, chunkValue]
      // setChunks((chunks) => [...chunks, chunkValue]);
    }
    let oldResponse = chunks.join('')

    let gptMessage = new Y.Map<any>()
    gptMessage.set('text', oldResponse)
    gptMessage.set('username', 'GPT')
    messages.push([gptMessage])
  }

  return (
    <div className="w-1/2 mx-auto border-white text-center mt-10">
      <div>Y-Sweet Chat</div>
      <ChatSize />
      <Username />
      <div className="bg-neutral-900 p-5 mt-4 h-96 overflow-y-scroll">
        {messages.map((message, i) => {
          return (
            <Message
              isSelf={presence.get(doc.clientID)?.username === message.get('username')}
              key={i}
              username={message.get('username')}
              text={message.get('text')}
            />
          )
        })}
      </div>
      <form
        onSubmit={async (e) => {
          e.preventDefault()

          let newMessage = new Y.Map<any>()
          newMessage.set('text', message)
          newMessage.set('username', presence.get(doc.clientID)?.username)
          messages.push([newMessage])

          setPresence({
            username: 'GPT',
            isTyping: true,
          })
          if (message.includes('@gpt')) {
            await getOpenAIResponse(message)
          }
          setPresence({
            username: 'GPT',
            isTyping: false,
          })
        }}
      >
        <input
          type="text"
          placeholder="Type your message..."
          className="w-full p-4 bg-neutral-700 caret-neutral-200 text-white"
          value={message}
          onChange={(e) => {
            setMessage(e.target.value)
          }}
          onFocus={() => {
            setPresence({
              username: usernameInput,
              isTyping: true,
            })
          }}
          onBlur={() => {
            setPresence({
              username: usernameInput,
              isTyping: false,
            })
          }}
        />
      </form>
      <div>
        {Array.from(presence.entries()).map(([key, value]) => {
          if (value.isTyping) {
            return <span key={key}>{value.username} is typing</span>
          }
        })}
      </div>
    </div>
  )
}

function ChatSize() {
    const presence = usePresence<Presence>()
    return(
        <span>There are {presence.size + 1} people in the chat</span>
    )
}

function Username() {
    const doc = useYDoc()
    const setPresence = usePresenceSetter<Presence>()
    const [usernameInput, setUsernameInput] = useState<string>('')

    useEffect(() => {
        setUsernameInput(`user${doc.clientID}`)
    }, [doc.clientID])

    return(
      <div className="flex items-center justify-center mt-2">
        <div className="w-4 h-4 bg-green-300 rounded-full mr-2" />
        <span>Connected as
            <form
            onSubmit={(e) => {
                e.preventDefault()
                setPresence({
                    username: usernameInput,
                    isTyping: false
                })
            }}
            >
        <input
          type="text"
          placeholder={`user${doc.clientID}`}
          className="w-full p-4 bg-neutral-700 caret-neutral-200 text-white"
          value={usernameInput}
          onChange={(e) => {
            setUsernameInput(e.target.value)
          }}
        />
        </form>
        </span>
      </div>
    )
}

interface MessageProps {
  username: string
  text?: string
  aiResponse?: JSX.Element
  isSelf: boolean
}

function Message(props: MessageProps) {
  const { username, text, isSelf, aiResponse } = props

  return (
    <div
      className="pb-4"
      style={{
        textAlign: isSelf ? 'right' : 'left',
      }}
    >
      <div className="text-sm pb-1">{username}</div>
      {
        <div
          className="px-4 py-2 rounded-xl inline-block"
          style={{ backgroundColor: isSelf ? 'green' : 'black' }}
        >
          {text && text}
          {aiResponse && aiResponse}
        </div>
      }
    </div>
  )
}
