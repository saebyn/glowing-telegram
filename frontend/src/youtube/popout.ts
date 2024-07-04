const proceedButton = document.getElementById("proceed-button");

if (!proceedButton) {
  throw new Error("Proceed button not found");
}
proceedButton.addEventListener("click", () => {
  // Redirect to YouTube login @ https://accounts.google.com/o/oauth2/auth
  // but forward all query parameters from this URL to the YouTube login URL
  const queryParams = new URLSearchParams(window.location.search);
  const baseUrl = import.meta.env.VITE_YOUTUBE_AUTH_URI;
  const redirectUrl = `${baseUrl}?${queryParams.toString()}`;
  window.location.href = redirectUrl;
});
