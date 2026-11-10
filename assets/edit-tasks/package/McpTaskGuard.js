include_file("McpDocumentContext.js");

function mcpClaimTask(readFn, writeFn, requestFilename, kind) {
	var request = readFn(requestFilename);
	var lease = readFn("task-lease.json");
	var status = readFn("status.json");
	var consumedRequest = writeFn(requestFilename, { id: null, ops: [] });
	var consumedLease = writeFn("task-lease.json", { id: null });
	var remainingRequest = readFn(requestFilename);
	var remainingLease = readFn("task-lease.json");
	var error = null;
	if (!consumedRequest || !consumedLease || !remainingRequest || remainingRequest.id !== null || !remainingLease || remainingLease.id !== null) {
		error = "could not consume EditTask request and lease";
	} else if (!request || typeof request.id !== "string" || !request.id || !Array.isArray(request.ops)) {
		error = "no edit request";
	} else if (!lease || lease.id !== request.id || lease.kind !== kind) {
		error = "no matching EditTask lease";
	} else if (!status || status.protocol !== 2 || status.closed || typeof status.session !== "string" || !status.session || status.session !== lease.session) {
		error = "EditTask bridge session is not current";
	} else if (request.expectedSession !== lease.session || typeof lease.document !== "string" || !lease.document || request.expectedDocument !== lease.document) {
		error = "EditTask request context does not match its lease";
	} else if (typeof lease.expiresAt !== "number" || !isFinite(lease.expiresAt) || lease.expiresAt <= Date.now()) {
		error = "EditTask lease expired";
	} else if ((lease.title !== null && typeof lease.title !== "string") || (lease.fileUrl !== null && typeof lease.fileUrl !== "string")) {
		error = "EditTask lease has no document identity";
	} else if (typeof Host === "undefined" || !Host.Objects || typeof Host.Objects.getObjectByUrl !== "function") {
		error = "active document is not available";
	} else if (lease.nativeDocument !== undefined) {
		try {
			var identity = lease.nativeDocument;
			if (!identity || typeof identity !== "object" || Array.isArray(identity)
				|| Object.keys(identity).sort().join(",") !== "context_api,token"
				|| identity.context_api !== "native_document_context_v1" || !mcpDocumentToken(identity.token)) throw new Error("invalid native task identity");
			var context = mcpReadNativeDocument(mcpNativeDocumentService());
			if (!context.open || context.token !== identity.token || context.title !== lease.title || context.fileUrl !== lease.fileUrl)
				throw new Error("native task document changed");
		} catch (_) { error = "native document does not match EditTask lease"; }
	} else {
		var manager = Host.Objects.getObjectByUrl("://studioapp/DocumentManager");
		var document = manager && typeof manager.activeDocument === "object" ? manager.activeDocument : null;
		var title = document && typeof document.title === "string" ? document.title : null;
		var fileUrl = document && document.path && typeof document.path.url === "string" ? document.path.url : null;
		if (!document || title !== lease.title || fileUrl !== lease.fileUrl) error = "active document does not match EditTask lease";
	}
	return { request: request, error: error };
}
