'use client'

import React, { useRef, useState } from 'react'
import { motion, useScroll, useTransform } from 'framer-motion'
import { Music, Users, Share2 } from 'lucide-react'

// Mock user data
interface User {
  id: number
  name: string
  color: string
  avatar: string
}

const MOCK_USERS: User[] = [
  { id: 1, name: 'Alex', color: 'hsl(262 83% 58%)', avatar: 'A' },
  { id: 2, name: 'Sam', color: 'hsl(142 76% 36%)', avatar: 'S' },
  { id: 3, name: 'Jordan', color: 'hsl(24 95% 53%)', avatar: 'J' },
  { id: 4, name: 'Riley', color: 'hsl(199 89% 48%)', avatar: 'R' },
]

// Mock queue tracks with user assignments
interface QueueTrackWithUser {
  trackId: string
  title: string
  artist: string
  album: string
  durationSeconds: number
  addedBy: User
  coverArtPath?: string
}

const MOCK_QUEUE: QueueTrackWithUser[] = [
  {
    trackId: '1',
    title: 'Neon Dreamscape',
    artist: 'Synthwave Collective',
    album: 'Digital Horizons',
    durationSeconds: 245,
    addedBy: MOCK_USERS[0],
  },
  {
    trackId: '2',
    title: 'Midnight Drive',
    artist: 'Retrowave Express',
    album: 'Night City',
    durationSeconds: 312,
    addedBy: MOCK_USERS[1],
  },
  {
    trackId: '3',
    title: 'Electric Soul',
    artist: 'Future Funk Band',
    album: 'Analog Dreams',
    durationSeconds: 198,
    addedBy: MOCK_USERS[2],
  },
  {
    trackId: '4',
    title: 'Cosmic Journey',
    artist: 'Space Groove',
    album: 'Beyond the Stars',
    durationSeconds: 267,
    addedBy: MOCK_USERS[0],
  },
  {
    trackId: '5',
    title: 'Urban Sunset',
    artist: 'City Lights',
    album: 'Metropolitan',
    durationSeconds: 223,
    addedBy: MOCK_USERS[3],
  },
  {
    trackId: '6',
    title: 'Digital Rain',
    artist: 'Cyber Dreams',
    album: 'Virtual Reality',
    durationSeconds: 289,
    addedBy: MOCK_USERS[1],
  },
]

export function MultiUserShowcase() {
  const containerRef = useRef<HTMLDivElement>(null)
  const cardRef = useRef<HTMLDivElement>(null)
  const [mousePosition, setMousePosition] = useState({ x: 0, y: 0 })
  const [isHovered, setIsHovered] = useState(false)

  const { scrollYProgress } = useScroll({
    target: containerRef,
    offset: ['start end', 'end start'],
  })

  // 3D rotation based on scroll
  const rotateX = useTransform(scrollYProgress, [0, 0.5, 1], [15, 0, -15])
  const rotateY = useTransform(scrollYProgress, [0, 0.5, 1], [-10, 0, 10])

  // Mouse move effect for 3D tilt
  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!cardRef.current) return
    const rect = cardRef.current.getBoundingClientRect()
    const x = (e.clientX - rect.left) / rect.width
    const y = (e.clientY - rect.top) / rect.height
    setMousePosition({ x, y })
  }

  const handleMouseLeave = () => {
    setIsHovered(false)
    setMousePosition({ x: 0.5, y: 0.5 })
  }

  const handleMouseEnter = () => {
    setIsHovered(true)
  }

  // Calculate tilt based on mouse position
  const tiltX = isHovered ? (mousePosition.y - 0.5) * -10 : 0
  const tiltY = isHovered ? (mousePosition.x - 0.5) * 10 : 0

  const formatDuration = (seconds: number) => {
    const mins = Math.floor(seconds / 60)
    const secs = Math.floor(seconds % 60)
    return `${mins}:${secs.toString().padStart(2, '0')}`
  }

  return (
    <div ref={containerRef} className="relative py-16 md:py-24">
      {/* Background network visualization */}
      <div className="absolute inset-0 overflow-hidden pointer-events-none">
        <svg className="absolute inset-0 w-full h-full opacity-10">
          {/* Connection lines between users */}
          {MOCK_USERS.map((user, i) => {
            const x1 = 20 + (i * 25) % 80
            const y1 = 20 + (i * 30) % 60
            const x2 = 20 + ((i + 1) * 25) % 80
            const y2 = 20 + ((i + 1) * 30) % 60
            return (
              <motion.line
                key={`line-${i}`}
                x1={`${x1}%`}
                y1={`${y1}%`}
                x2={`${x2}%`}
                y2={`${y2}%`}
                stroke={user.color}
                strokeWidth="2"
                strokeDasharray="5,5"
                initial={{ pathLength: 0 }}
                animate={{ pathLength: 1 }}
                transition={{ duration: 2, delay: i * 0.2 }}
              />
            )
          })}
        </svg>
      </div>

      {/* 3D Card Container */}
      <motion.div
        style={{
          perspective: '1000px',
          transformStyle: 'preserve-3d',
        }}
        className="max-w-5xl mx-auto px-6"
      >
        <motion.div
          ref={cardRef}
          style={{
            rotateX: isHovered ? tiltX : rotateX,
            rotateY: isHovered ? tiltY : rotateY,
            transformStyle: 'preserve-3d',
          }}
          transition={{ type: 'spring', stiffness: 300, damping: 30 }}
          onMouseMove={handleMouseMove}
          onMouseLeave={handleMouseLeave}
          onMouseEnter={handleMouseEnter}
          className="relative"
        >
          {/* Main Card with grain texture */}
          <div
            className="relative grain-visible rounded-2xl overflow-hidden"
            style={{
              backgroundColor: 'hsl(var(--card))',
              border: '2px solid hsl(var(--border))',
              boxShadow: '0 25px 50px -12px hsl(var(--primary) / 0.25)',
            }}
          >
            {/* Gradient overlay */}
            <div
              className="absolute inset-0 pointer-events-none"
              style={{
                background:
                  'radial-gradient(ellipse 80% 50% at 50% 0%, hsl(var(--primary) / 0.15) 0%, transparent 60%)',
              }}
            />

            {/* Content Grid */}
            <div className="relative grid grid-cols-1 lg:grid-cols-2 gap-8 p-8 md:p-12">
              {/* Left Side - User Profiles */}
              <div className="space-y-6">
                <div className="flex items-center gap-2 mb-6">
                  <Users className="w-6 h-6" style={{ color: 'hsl(var(--primary))' }} />
                  <h3
                    className="text-2xl font-bold"
                    style={{ color: 'hsl(var(--foreground))' }}
                  >
                    Connected Users
                  </h3>
                </div>

                {/* User avatars and stats */}
                <div className="grid grid-cols-2 gap-4">
                  {MOCK_USERS.map((user, i) => (
                    <motion.div
                      key={user.id}
                      initial={{ opacity: 0, y: 20 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={{ delay: i * 0.1 }}
                      className="relative p-4 rounded-xl"
                      style={{
                        backgroundColor: 'hsl(var(--muted) / 0.3)',
                        border: `2px solid ${user.color}`,
                      }}
                    >
                      {/* Pulsing indicator */}
                      <div className="absolute -top-1 -right-1">
                        <span className="relative flex h-3 w-3">
                          <span
                            className="animate-ping absolute inline-flex h-full w-full rounded-full opacity-75"
                            style={{ backgroundColor: user.color }}
                          />
                          <span
                            className="relative inline-flex rounded-full h-3 w-3"
                            style={{ backgroundColor: user.color }}
                          />
                        </span>
                      </div>

                      {/* Avatar */}
                      <div
                        className="w-12 h-12 rounded-full flex items-center justify-center text-white font-bold text-lg mb-2"
                        style={{ backgroundColor: user.color }}
                      >
                        {user.avatar}
                      </div>

                      {/* User info */}
                      <div>
                        <div className="font-medium" style={{ color: 'hsl(var(--foreground))' }}>
                          {user.name}
                        </div>
                        <div className="text-xs" style={{ color: 'hsl(var(--muted-foreground))' }}>
                          {MOCK_QUEUE.filter((t) => t.addedBy.id === user.id).length} tracks
                        </div>
                      </div>
                    </motion.div>
                  ))}
                </div>

                {/* Collaboration Stats */}
                <div
                  className="p-4 rounded-xl"
                  style={{
                    backgroundColor: 'hsl(var(--primary) / 0.1)',
                    border: '1px solid hsl(var(--primary) / 0.3)',
                  }}
                >
                  <div className="flex items-center gap-2 mb-2">
                    <Share2 className="w-4 h-4" style={{ color: 'hsl(var(--primary))' }} />
                    <span className="text-sm font-medium" style={{ color: 'hsl(var(--foreground))' }}>
                      Shared Library
                    </span>
                  </div>
                  <div className="text-xs" style={{ color: 'hsl(var(--muted-foreground))' }}>
                    Everyone can add tracks, create playlists, and enjoy music together
                  </div>
                </div>
              </div>

              {/* Right Side - Shared Queue */}
              <div className="space-y-4">
                <div className="flex items-center gap-2 mb-4">
                  <Music className="w-6 h-6" style={{ color: 'hsl(var(--primary))' }} />
                  <h3
                    className="text-2xl font-bold"
                    style={{ color: 'hsl(var(--foreground))' }}
                  >
                    Shared Queue
                  </h3>
                </div>

                {/* Queue items */}
                <div className="space-y-2 max-h-[400px] overflow-y-auto pr-2">
                  {MOCK_QUEUE.map((track, i) => (
                    <motion.div
                      key={track.trackId}
                      initial={{ opacity: 0, x: 20 }}
                      animate={{ opacity: 1, x: 0 }}
                      transition={{ delay: 0.3 + i * 0.05 }}
                      className="relative p-3 rounded-lg group hover:bg-foreground/[var(--hover-bg-opacity)] transition-all cursor-pointer"
                      style={{
                        backgroundColor: 'hsl(var(--muted) / 0.2)',
                        borderLeft: `3px solid ${track.addedBy.color}`,
                      }}
                    >
                      <div className="flex items-center gap-3">
                        {/* Album art placeholder */}
                        <div
                          className="w-10 h-10 rounded flex-shrink-0 flex items-center justify-center"
                          style={{ backgroundColor: 'hsl(var(--muted))' }}
                        >
                          <Music className="w-5 h-5" style={{ color: 'hsl(var(--muted-foreground))' }} />
                        </div>

                        {/* Track info */}
                        <div className="flex-1 min-w-0">
                          <div className="font-medium text-sm truncate" style={{ color: 'hsl(var(--foreground))' }}>
                            {track.title}
                          </div>
                          <div className="text-xs truncate" style={{ color: 'hsl(var(--muted-foreground))' }}>
                            {track.artist}
                          </div>
                        </div>

                        {/* Duration */}
                        <div
                          className="text-xs font-mono flex-shrink-0"
                          style={{ color: 'hsl(var(--muted-foreground))' }}
                        >
                          {formatDuration(track.durationSeconds)}
                        </div>
                      </div>

                      {/* Added by badge */}
                      <div className="absolute -top-1 -right-1 opacity-0 group-hover:opacity-100 transition-opacity">
                        <div
                          className="px-2 py-0.5 rounded-full text-[10px] font-medium text-white"
                          style={{ backgroundColor: track.addedBy.color }}
                        >
                          {track.addedBy.name}
                        </div>
                      </div>
                    </motion.div>
                  ))}
                </div>

                {/* Queue stats */}
                <div
                  className="text-xs text-center py-2"
                  style={{ color: 'hsl(var(--muted-foreground))' }}
                >
                  {MOCK_QUEUE.length} tracks • {Math.floor(MOCK_QUEUE.reduce((acc, t) => acc + t.durationSeconds, 0) / 60)} minutes
                </div>
              </div>
            </div>

            {/* Network effect overlay */}
            <div className="absolute inset-0 pointer-events-none overflow-hidden opacity-20">
              {MOCK_USERS.map((user, i) => (
                <motion.div
                  key={`dot-${user.id}`}
                  className="absolute w-2 h-2 rounded-full"
                  style={{
                    backgroundColor: user.color,
                    left: `${20 + (i * 25) % 80}%`,
                    top: `${20 + (i * 30) % 60}%`,
                  }}
                  animate={{
                    scale: [1, 1.5, 1],
                    opacity: [0.5, 1, 0.5],
                  }}
                  transition={{
                    duration: 2,
                    repeat: Infinity,
                    delay: i * 0.5,
                  }}
                />
              ))}
            </div>
          </div>

          {/* 3D depth effect shadow */}
          <div
            className="absolute -inset-4 -z-10 blur-2xl opacity-30"
            style={{
              background: `radial-gradient(ellipse 100% 100% at 50% 50%, hsl(var(--primary) / 0.4) 0%, transparent 70%)`,
            }}
          />
        </motion.div>
      </motion.div>

      {/* Feature highlights below card */}
      <motion.div
        initial={{ opacity: 0, y: 30 }}
        whileInView={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.5 }}
        viewport={{ once: true }}
        className="max-w-5xl mx-auto px-6 mt-12"
      >
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          {[
            {
              title: 'Separate Profiles',
              description: 'Each user gets their own playlists, history, and preferences',
              icon: Users,
            },
            {
              title: 'Shared Discovery',
              description: "See what others are listening to and discover through your circle's taste",
              icon: Share2,
            },
            {
              title: 'Collaborative Queues',
              description: 'Build queues together - everyone can contribute to the vibe',
              icon: Music,
            },
          ].map((feature, i) => (
            <motion.div
              key={i}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.6 + i * 0.1 }}
              viewport={{ once: true }}
              className="p-6 rounded-xl text-center"
              style={{
                backgroundColor: 'hsl(var(--card))',
                border: '1px solid hsl(var(--border))',
              }}
            >
              <feature.icon
                className="w-8 h-8 mx-auto mb-3"
                style={{ color: 'hsl(var(--primary))' }}
              />
              <h4 className="font-semibold mb-2" style={{ color: 'hsl(var(--foreground))' }}>
                {feature.title}
              </h4>
              <p className="text-sm" style={{ color: 'hsl(var(--muted-foreground))' }}>
                {feature.description}
              </p>
            </motion.div>
          ))}
        </div>
      </motion.div>
    </div>
  )
}
