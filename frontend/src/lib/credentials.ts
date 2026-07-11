// Login and password-change here are plain fetch calls with no full page
// navigation, which is the signal browsers normally rely on to decide "this
// login succeeded, offer to save the password". Explicitly telling the
// Credential Management API is the documented workaround for SPAs. Chrome/Edge
// support it; Firefox/Safari don't define window.PasswordCredential, so this
// is a no-op there and falls back to their own (weaker) autofill heuristics.
export async function storePasswordCredential(id: string, password: string): Promise<void> {
  const PasswordCredentialCtor = (
    window as unknown as { PasswordCredential?: new (data: unknown) => Credential }
  ).PasswordCredential
  if (!PasswordCredentialCtor || !navigator.credentials?.store) return

  try {
    const credential = new PasswordCredentialCtor({ id, password })
    await navigator.credentials.store(credential)
  } catch {
    // best-effort convenience only; never block the login/password-change flow
  }
}
