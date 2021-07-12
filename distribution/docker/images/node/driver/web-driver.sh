#!/usr/bin/env bash
GECKO_VERSION=0.29.1
CHROME_VERSION=91.0.4472.101
PREV_PWD=$(pwd)
TMP_DIR=$(mktemp -d)

cd $TMP_DIR

# TODO We might get a desync of browser and driver version since the drivers have an explicit version specified while the browsers do not

function geckodriver {
	# Install driver
	wget --no-check-certificate https://github.com/mozilla/geckodriver/releases/download/v${GECKO_VERSION}/geckodriver-v${GECKO_VERSION}-linux64.tar.gz
	tar xzf geckodriver*.tar.gz
	chmod +x geckodriver
	mv geckodriver /usr/bin

	# Install firefox
	install_packages firefox-esr

	echo 'export DRIVER="/usr/bin/geckodriver"' >> /env.sh
	echo 'export DRIVER_VARIANT="firefox"' >> /env.sh
	echo "export BROWSER_VERSION=\"$(firefox -version | cut -d " " -f 3 )\"" >> /env.sh
}

function chromedriver {
	# Install driver
	wget --no-check-certificate https://chromedriver.storage.googleapis.com/${CHROME_VERSION}/chromedriver_linux64.zip
	unzip chromedriver_linux64.zip
	chmod +x chromedriver
	mv chromedriver /usr/bin

	# Install chrome
	wget --no-check-certificate -q -O - https://dl-ssl.google.com/linux/linux_signing_key.pub | apt-key add -
	echo 'deb [arch=amd64] http://dl.google.com/linux/chrome/deb/ stable main' > /etc/apt/sources.list.d/google-chrome.list
	install_packages google-chrome-stable libnss3

	echo 'export DRIVER="/usr/bin/chromedriver"' >> /env.sh
	echo 'export DRIVER_VARIANT="chrome"' >> /env.sh
	echo "export BROWSER_VERSION=\"$(/usr/bin/google-chrome -version | awk '{ print $3 }')\"" >> /env.sh

	# Wrap the chrome binary so we run it with --no-sandbox
	$PREV_PWD/wrap-chrome.sh
}

case "$1" in
	firefox)
		geckodriver
		;;
	chrome)
		chromedriver
		;;
esac

cd $PREV_PWD
rm -rf $TMP_DIR
