# Render progress

Streamosaic render job progress is the percentage of the normalized output timeline encoded by FFmpeg, based on its reported video frame count at 60 frames per second. It is rounded down and capped at 99%. It does not estimate download or upload progress: the job can remain at 0% while inputs download, and at 99% while the output uploads. The completion transaction sets 100% only after a successful upload.

The worker publishes increasing values at most once every five seconds, scoped to the running render job, episode, tenant and assigned Batch job. A failed progress write is logged and does not fail the render. Missing or malformed FFmpeg progress is ignored; a render can therefore complete successfully without intermediate updates.
