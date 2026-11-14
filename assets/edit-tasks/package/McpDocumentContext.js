function mcpNativeDocumentService() {
	if (typeof Host === "undefined" || !Host.Objects) return null;
	var objects = Host.Objects;
	var service = typeof objects.getObjectByName === "function" ? objects.getObjectByName("FenderMcpNative") : null;
	if (!service && typeof objects.getObjectByUrl === "function") service = objects.getObjectByUrl("object://FenderMcpNative/", true);
	return service && typeof service.documentContext === "function" ? service : null;
}

function mcpDocumentToken(token) {
	if (typeof token !== "string") return null;
	var match = /^ndc1:([0-9a-f]{16}):([1-9][0-9]{0,19})$/.exec(token);
	if (!match || match[1] === "0000000000000000"
		|| (match[2].length === 20 && match[2] > "18446744073709551615")) return null;
	return { source: match[1], generation: match[2] };
}

function mcpReadNativeDocument(service) {
	if (!service || typeof service.documentContext !== "function") throw new Error("native document context is unavailable");
	var text = service.documentContext();
	if (typeof text !== "string" || !text || text.length > 262144) throw new Error("invalid native document response");
	var value = JSON.parse(text);
	if (!value || typeof value !== "object" || Array.isArray(value)
		|| Object.keys(value).sort().join(",") !== "api,context_api,fileUrl,ok,open,title,token"
		|| value.ok !== true || value.api !== "fender-mcp-native-v1" || value.context_api !== "native_document_context_v1"
		|| !mcpDocumentToken(value.token) || typeof value.open !== "boolean"
		|| (value.title !== null && (typeof value.title !== "string" || value.title.length > 4096 || value.title.indexOf("\0") >= 0))
		|| (value.fileUrl !== null && (typeof value.fileUrl !== "string" || value.fileUrl.length > 16384 || value.fileUrl.indexOf("\0") >= 0))
		|| (!value.open && (value.title !== null || value.fileUrl !== null))) throw new Error("native document context failed validation");
	return value;
}
