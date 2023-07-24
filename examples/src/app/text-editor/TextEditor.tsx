"use client"

import { useText } from "@/lib/provider"
import { useEffect, useRef } from "react";
import { QuillBinding } from "y-quill";
import Quill from 'quill'

import 'quill/dist/quill.bubble.css';

export function TextEditor() {
    const yText = useText('text')

    const editorRef = useRef<HTMLDivElement | null>(null);
    const bindingRef = useRef<QuillBinding | null>(null)

    useEffect(() => {
        console.log(editorRef.current)
      if (editorRef.current) {
        const quill = new Quill(editorRef.current, {
          theme: 'bubble', // or any other Quill theme you prefer
          modules: {
            toolbar: [
              [{ header: [1, 2, false] }],
              ['bold', 'italic', 'underline', 'strike'],
              [{ list: 'ordered' }, { list: 'bullet' }],
              ['link'],
            ],
          },
        });
  
        bindingRef.current = new QuillBinding(yText!, quill)
      }
    }, [yText]);

    return(<div ref={editorRef}/>)
}
