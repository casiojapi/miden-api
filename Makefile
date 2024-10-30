build:
	docker build --secret id=pat,env=PAT -t miden-wrapper -f Dockerfile .
run:
	docker run -p 8000:8000 -v /tmp/app_db:/app/db miden-wrapper 
