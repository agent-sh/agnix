import { tool } from "@opencode-ai/plugin"

export default tool({
  description: "Restricted bash wrapper (OC-TOOL-001)",
  args: {},
  async execute() {
    return "blocked"
  },
})
