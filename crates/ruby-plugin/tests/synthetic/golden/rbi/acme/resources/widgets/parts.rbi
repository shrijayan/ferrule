# typed: strong

module Acme
  module Resources
    class Widgets
      class Parts
        # Retrieve one part of a widget
        sig { params(part_number: Integer, widget_id: String, request_options: Acme::RequestOptions::OrHash).returns(Acme::Models::Widgets::PartRetrieveResponse) }
        def retrieve(part_number, widget_id:, request_options: {})
        end

        # @api private
        sig { params(client: Acme::Client).returns(T.attached_class) }
        def self.new(client:)
        end
      end
    end
  end
end
