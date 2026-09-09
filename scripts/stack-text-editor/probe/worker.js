// One bounded request/response, local module asset; no source document or remote fetch.
self.onmessage = ({ data }) => {
  if (data !== 'ready') return;
  self.postMessage('ready');
};
