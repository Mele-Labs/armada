// The phone fixture's pictures and video, which the `thread-media` scenario plays. Vite answers each with its URL.
declare module "*.jpg" {
  const url: string;
  export default url;
}
declare module "*.mp4" {
  const url: string;
  export default url;
}
