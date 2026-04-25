export const toBase64 = (file: File): Promise<string> =>
  new Promise((resolve, reject) => {
    const reader = new FileReader()

    reader.onload = () => {
      const result = reader.result
      if (typeof result !== 'string') {
        reject(new Error('Could not read file as Base64'))
        return
      }

      const [, base64] = result.split(',')
      resolve(base64)
    }

    reader.onerror = () => reject(reader.error ?? new Error('Failed to read file'))
    reader.readAsDataURL(file)
  })

export const sha256Hex = async (file: File): Promise<string> => {
  const buffer = await file.arrayBuffer()
  const digest = await crypto.subtle.digest('SHA-256', buffer)
  return [...new Uint8Array(digest)]
    .map((byte) => byte.toString(16).padStart(2, '0'))
    .join('')
}
