"use client"

import { useText, useAwareness } from "@/lib/provider"
import { useEffect, useRef } from "react";
import { QuillBinding } from "y-quill";
import Quill from 'quill'
import QuillCursors from 'quill-cursors'

import 'quill/dist/quill.snow.css';

export function TextEditor() {
    const yText = useText('text')
    const awareness = useAwareness()
    const editorRef = useRef<HTMLDivElement | null>(null);
    const bindingRef = useRef<QuillBinding | null>(null)

    useEffect(() => {
        if (bindingRef.current !== null) {
            return
        }
      if (editorRef.current && awareness) {
        Quill.register('modules/cursors', QuillCursors);
        const quill = new Quill(editorRef.current, {
          theme: 'snow', // or any other Quill theme you prefer
          modules: {
            cursors: true,
            toolbar: [
              [{ header: [1, 2, false] }],
              ['bold', 'italic', 'underline', 'strike'],
              [{ list: 'ordered' }, { list: 'bullet' }],
              ['link'],
            ],
          },
          
        });
        console.log(quill.getModule('cursors'))
        bindingRef.current = new QuillBinding(yText!, quill, awareness!)
        console.log(bindingRef.current)
      }
    }, [yText, awareness]);



    return(
    <div >
        <h1>A Collaborative Text Editor</h1>
        <div ref={editorRef}/>
    </div>
    )
}
