#!/bin/bash -ex

gem install rubygems-update -v 3.4.22
update_rubygems
bundle install
bundle exec jekyll build

