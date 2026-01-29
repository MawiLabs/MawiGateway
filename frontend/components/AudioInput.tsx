'use client'

import { useState, useRef, useEffect } from 'react'

interface AudioInputProps {
    onAudioReady: (blob: Blob) => void
    onClear: () => void
    clearTrigger?: number  // Change this value to trigger clear
}

export function AudioInput({ onAudioReady, onClear, clearTrigger }: AudioInputProps) {
    const [isRecording, setIsRecording] = useState(false)
    const [audioBlob, setAudioBlob] = useState<Blob | null>(null)
    const [audioURL, setAudioURL] = useState<string | null>(null)
    const mediaRecorderRef = useRef<MediaRecorder | null>(null)
    const chunksRef = useRef<Blob[]>([])

    // Clear audio when parent triggers clear
    useEffect(() => {
        if (clearTrigger !== undefined && clearTrigger > 0) {
            setAudioBlob(null)
            setAudioURL(null)
        }
    }, [clearTrigger])

    const startRecording = async () => {
        try {
            const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
            const mediaRecorder = new MediaRecorder(stream)
            mediaRecorderRef.current = mediaRecorder
            chunksRef.current = []

            mediaRecorder.ondataavailable = (e) => {
                if (e.data.size > 0) {
                    chunksRef.current.push(e.data)
                }
            }

            mediaRecorder.onstop = () => {
                const blob = new Blob(chunksRef.current, { type: 'audio/webm' })
                setAudioBlob(blob)
                setAudioURL(URL.createObjectURL(blob))
                onAudioReady(blob)

                // Stop all tracks
                stream.getTracks().forEach(track => track.stop())
            }

            mediaRecorder.start()
            setIsRecording(true)
        } catch (error) {
            console.error('Failed to start recording:', error)
            alert('Microphone access denied or not available')
        }
    }

    const stopRecording = () => {
        if (mediaRecorderRef.current && isRecording) {
            mediaRecorderRef.current.stop()
            setIsRecording(false)
        }
    }

    const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0]
        if (file) {
            setAudioBlob(file)
            setAudioURL(URL.createObjectURL(file))
            onAudioReady(file)
        }
    }

    const clearAudio = () => {
        setAudioBlob(null)
        setAudioURL(null)
        onClear()
    }

    return (
        <div className="space-y-4">
            {/* Recording Controls */}
            <div className="flex gap-3">
                {!isRecording && !audioBlob && (
                    <>
                        <button
                            onClick={startRecording}
                            className="flex-1 px-4 py-2 bg-red-500/20 border border-red-400/30 rounded-xl text-red-400 hover:bg-red-500/30 transition-all flex items-center justify-center gap-2">
                            <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                                <path fillRule="evenodd" d="M7 4a3 3 0 016 0v4a3 3 0 11-6 0V4zm4 10.93A7.001 7.001 0 0017 8a1 1 0 10-2 0A5 5 0 015 8a1 1 0 00-2 0 7.001 7.001 0 006 6.93V17H6a1 1 0 100 2h8a1 1 0 100-2h-3v-2.07z" clipRule="evenodd" />
                            </svg>
                            Record Audio
                        </button>

                        <label className="flex-1 px-4 py-2 bg-cyan-500/20 border border-cyan-400/30 rounded-xl text-cyan-400 hover:bg-cyan-500/30 transition-all flex items-center justify-center gap-2 cursor-pointer">
                            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                            </svg>
                            Upload File
                            <input
                                type="file"
                                accept="audio/*"
                                onChange={handleFileUpload}
                                className="hidden"
                            />
                        </label>
                    </>
                )}

                {isRecording && (
                    <button
                        onClick={stopRecording}
                        className="flex-1 px-4 py-2 bg-red-500/40 border border-red-400/50 rounded-xl text-red-300 hover:bg-red-500/50 transition-all flex items-center justify-center gap-2 animate-pulse">
                        <span className="w-3 h-3 bg-red-500 rounded-full"></span>
                        Stop Recording
                    </button>
                )}

                {audioBlob && !isRecording && (
                    <button
                        onClick={clearAudio}
                        className="px-4 py-2 bg-slate-700/50 border border-slate-600/50 rounded-xl text-slate-400 hover:bg-slate-700/70 transition-all">
                        Clear
                    </button>
                )}
            </div>

            {/* Audio Preview */}
            {audioURL && (
                <div className="p-4 bg-white/5 border border-white/10 rounded-xl">
                    <p className="text-xs text-slate-400 mb-2">Audio Preview:</p>
                    <audio
                        src={audioURL}
                        controls
                        className="w-full"
                    />
                    <p className="text-xs text-slate-500 mt-2">
                        Size: {(audioBlob!.size / 1024).toFixed(2)} KB
                    </p>
                </div>
            )}
        </div>
    )
}
