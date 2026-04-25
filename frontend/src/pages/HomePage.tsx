import { motion } from 'framer-motion'
import { HardDriveDownload, Layers, Server } from 'lucide-react'
import heroImg from '../assets/hero.png'

export const HomePage = () => {
  return (
    <section className="space-y-6">
      <div className="glass-card overflow-hidden rounded-3xl border border-white/15 p-8 md:p-12">
        <div className="grid items-center gap-8 md:grid-cols-2">
          <div>
            <p className="mb-3 inline-flex rounded-full bg-cyan-300/20 px-3 py-1 text-xs font-semibold uppercase tracking-[0.16em] text-cyan-100">
              Cross-Platform File Sync
            </p>
            <h1 className="font-display text-4xl font-semibold leading-tight md:text-5xl">
              Control your files like a local cloud command center.
            </h1>
            <p className="mt-4 max-w-xl text-sm text-white/70 md:text-base">
              Anywhere Door combines a Rust service agent and FastAPI backend so you can watch
              folders, sync events, and manage uploads from one dashboard.
            </p>
            <div className="mt-6 flex flex-wrap gap-3 text-xs text-white/70">
              <span className="rounded-full bg-white/10 px-3 py-1">Rust Agent</span>
              <span className="rounded-full bg-white/10 px-3 py-1">FastAPI Backend</span>
              <span className="rounded-full bg-white/10 px-3 py-1">HMAC + JWT Security</span>
            </div>
          </div>
          <div className="relative flex justify-center">
            <img
              src={heroImg}
              alt="Anywhere Door stacked logo"
              className="floating-image w-56 max-w-full md:w-72"
            />
          </div>
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        {[
          {
            title: 'What this app does',
            icon: <Layers size={18} />,
            body: 'Monitors local folders, streams filesystem events, and syncs files to your secure backend.',
          },
          {
            title: 'Agent download / build',
            icon: <HardDriveDownload size={18} />,
            body: 'Build with cargo release and install service scripts for Windows or Linux.',
          },
          {
            title: 'Setup flow',
            icon: <Server size={18} />,
            body: 'Login, register device, select watch roots, then run headlessly as an OS service.',
          },
        ].map((item, idx) => (
          <motion.article
            key={item.title}
            className="glass-card rounded-2xl border border-white/10 p-5"
            initial={{ opacity: 0, y: 18 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 + idx * 0.12, duration: 0.28 }}
          >
            <div className="mb-3 inline-flex rounded-xl bg-white/10 p-2">{item.icon}</div>
            <h2 className="font-display text-lg font-medium">{item.title}</h2>
            <p className="mt-2 text-sm text-white/70">{item.body}</p>
          </motion.article>
        ))}
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <article className="glass-card rounded-2xl border border-white/10 p-6">
          <h3 className="font-display text-xl">Windows setup</h3>
          <pre className="mt-3 overflow-auto rounded-xl bg-black/40 p-4 text-xs text-cyan-100">
{`cd anywhere_door_agent
cargo build --release
# Run PowerShell as Administrator
.\\scripts\\install-windows-service.ps1`}
          </pre>
        </article>
        <article className="glass-card rounded-2xl border border-white/10 p-6">
          <h3 className="font-display text-xl">Linux setup</h3>
          <pre className="mt-3 overflow-auto rounded-xl bg-black/40 p-4 text-xs text-cyan-100">
{`cd anywhere_door_agent
cargo build --release
sudo ./scripts/install-systemd.sh`}
          </pre>
        </article>
      </div>
    </section>
  )
}
