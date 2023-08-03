"use client"
import {useState} from "react"

export default function CopyLink(){
    const [buttonState, setButtonState] = useState("Copy link")

    const copyLinkToClipboard = () => {
        const currentPageURL = window.location.href;

        // Copy the link to the clipboard
        navigator.clipboard.writeText(currentPageURL)
          .then(() => {
            // Optionally, provide some user feedback (e.g., display a message)
            setButtonState("Copied!")
            // Or, you can change the button text to indicate that it has been copied:
            // document.getElementById('copyLinkButton').innerText = 'Copied!';
          })
          .catch((error) => {
            console.error('Failed to copy link: ', error);
          });
      };

    return(
        <div className="p-6 flex flex-col text-left rounded-lg bg-white/50 border border-white/80 shadow-sm">
            <span className="text-pink-900 pb-2">Open this doc in a new window or share with a friend:</span>
            <button className="w-28 py-2 rounded-lg bg-pink-950 text-white transition-all hover:bg-pink-900" onClick={copyLinkToClipboard}>{buttonState}</button>
        </div>
    )
}
