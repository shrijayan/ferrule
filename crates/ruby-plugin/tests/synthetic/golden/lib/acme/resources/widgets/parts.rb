# frozen_string_literal: true

module Acme
  module Resources
    class Widgets
      class Parts
        # Retrieve one part of a widget
        #
        # @overload retrieve(part_number, widget_id:, request_options: {})
        #
        # @param part_number [Integer]
        # @param widget_id [String]
        # @param request_options [Acme::RequestOptions, Hash{Symbol=>Object}, nil]
        #
        # @return [Acme::Models::Widgets::PartRetrieveResponse]
        #
        # @see Acme::Models::Widgets::PartRetrieveParams
        def retrieve(part_number, params)
          parsed, options = Acme::Widgets::PartRetrieveParams.dump_request(params)
          widget_id =
            parsed.delete(:widget_id) do
              raise ArgumentError.new("missing required path argument #{_1}")
            end
          @client.request(
            method: :get,
            path: ["v1/widgets/%1$s/parts/%2$s", widget_id, part_number],
            model: Acme::Models::Widgets::PartRetrieveResponse,
            options: options
          )
        end

        # @api private
        #
        # @param client [Acme::Client]
        def initialize(client:)
          @client = client
        end
      end
    end
  end
end
