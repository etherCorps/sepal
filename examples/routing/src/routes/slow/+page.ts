// A load that takes a while. The previous page stays up until it is done.
export async function load() {
	await new Promise((done) => setTimeout(done, 200));
	return { waited: 200 };
}
