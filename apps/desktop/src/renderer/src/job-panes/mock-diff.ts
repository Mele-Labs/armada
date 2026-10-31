// **The mock's** diffs: what a Changes tab draws until Fleet serves a Job's own, one patch for each place the changes can be read from. A patch as git writes it.

export const MOCK_PATCH = `diff --git a/crates/fleet/src/lease.rs b/crates/fleet/src/lease.rs
index 4be1c3a..9d02f7e 100644
--- a/crates/fleet/src/lease.rs
+++ b/crates/fleet/src/lease.rs
@@ -41,9 +41,16 @@ impl Pool {
     pub fn lease(&self, job: &JobId) -> Result<Slot, LeaseError> {
-        let slot = self.free.pop().ok_or(LeaseError::Contended)?;
-        Ok(slot)
+        for attempt in 0..MAX_ATTEMPTS {
+            if let Some(slot) = self.free.pop() {
+                return Ok(slot);
+            }
+            std::thread::sleep(backoff(attempt));
+        }
+        Err(LeaseError::Contended)
     }
 }
diff --git a/apps/desktop/src/renderer/src/cockpit/deck/BayRows.tsx b/apps/desktop/src/renderer/src/cockpit/deck/BayRows.tsx
index 71a0e22..c3b9a10 100644
--- a/apps/desktop/src/renderer/src/cockpit/deck/BayRows.tsx
+++ b/apps/desktop/src/renderer/src/cockpit/deck/BayRows.tsx
@@ -88,6 +88,7 @@ export function BayRow({ bay }: { bay: Bay }) {
   return (
     <li className="deck-bay">
+      <AttemptMark attempt={bay.attempt} />
       <span className="deck-bay__title">{bay.title}</span>
     </li>
   );
`;

export const MOCK_PATCH_TOOLTIP = `diff --git a/apps/desktop/src/renderer/src/cockpit/deck/BayRows.tsx b/apps/desktop/src/renderer/src/cockpit/deck/BayRows.tsx
index c3b9a10..5e41d0b 100644
--- a/apps/desktop/src/renderer/src/cockpit/deck/BayRows.tsx
+++ b/apps/desktop/src/renderer/src/cockpit/deck/BayRows.tsx
@@ -88,7 +88,10 @@ export function BayRow({ bay }: { bay: Bay }) {
   return (
     <li className="deck-bay">
-      <AttemptMark attempt={bay.attempt} />
+      <Tooltip label={\`Attempt \${bay.attempt}\`}>
+        <AttemptMark attempt={bay.attempt} />
+      </Tooltip>
       <span className="deck-bay__title">{bay.title}</span>
     </li>
   );
`;

export const MOCK_PATCH_LOG = `diff --git a/crates/fleet/src/log_reader.rs b/crates/fleet/src/log_reader.rs
index 0b7e4d1..a41c9f3 100644
--- a/crates/fleet/src/log_reader.rs
+++ b/crates/fleet/src/log_reader.rs
@@ -12,6 +12,8 @@ impl LogReader {
     pub fn read(&mut self) -> io::Result<Vec<u8>> {
-        let mut buffer = Vec::new();
+        let mut buffer = Vec::with_capacity(CAP);
         self.file.read_to_end(&mut buffer)?;
+        buffer.truncate(CAP);
         Ok(buffer)
     }
 }
`;
