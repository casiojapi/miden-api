build:
	docker build --no-cache-filter=app-builder --secret id=pat,env=PAT -t miden-wrapper:latest --build-arg BUILD_BRANCH=main -f Dockerfile .
	NEXT_VERSION=$$(( $$(cat .version) + 1 )) && docker tag miden-wrapper:latest franbcki/miden-wrapper:0.$${NEXT_VERSION} && docker tag miden-wrapper:latest franbcki/miden-wrapper:latest && echo $${NEXT_VERSION} > .version

run:
	docker run -p 8000:8000 -v /tmp/app_db:/app/db miden-wrapper 
